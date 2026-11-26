//! Placing, taking, dropping and resetting. Only the authoritative machine (a server, or a game
//! without a network) changes pickups; it never rewinds, so nothing here has to survive a replay.

use crate::config::{LootConfig, PickupKind};
use crate::state::{Pickup, PickupOrigin};
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, NetPose, RemoveNetEntity, RetiredEntity};
use retreat_gameplay_api::{
    CharacterSize, GrantRequested, Health, Inventory, ItemCatalog, ItemStack, ItemsLost,
    MatchRestarted, MatchState, PickupSpots, PlayerId, PlayerRespawned, ticks_from_seconds,
};

/// A pickup floats this far above its ground.
pub const HOVER_HEIGHT: f32 = 0.5;
/// A player takes only what lies about level with its feet (not from a platform above or below).
const LEVEL_TOLERANCE: f32 = 1.0;

fn contents_of(kind: &PickupKind) -> Vec<ItemStack> {
    kind.contents
        .iter()
        .map(|(item, count)| ItemStack {
            item: item.clone(),
            count: *count,
        })
        .collect()
}

/// Puts a pickup at every spot that has none yet.
pub(crate) fn place_spot_pickups(
    spots: Res<PickupSpots>,
    config: Res<LootConfig>,
    client: Res<NetLocalClient>,
    names: Query<&Name, With<Pickup>>,
    mut commands: Commands,
) {
    if client.0.is_some() {
        return;
    }
    for spot in &spots.0 {
        let Some(kind) = config.kind(&spot.kind) else {
            continue;
        };
        let name = format!("pickup/{}", spot.name);
        if names.iter().any(|found| found.as_str() == name) {
            continue;
        }
        commands.spawn((
            Name::new(name),
            Pickup {
                contents: contents_of(kind),
                origin: PickupOrigin::Spot {
                    kind: spot.kind.clone(),
                    respawn_ticks_left: 0,
                },
            },
            NetPose::at(spot.feet + Vec3::Y * HOVER_HEIGHT),
        ));
    }
}

/// One tick of the respawn delays and of the drops' time. Clients count too (a timer they can
/// predict, so it costs no corrections); only the authority removes.
pub(crate) fn advance_timers(
    mut pickups: Query<(Entity, &mut Pickup), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    mut commands: Commands,
) {
    for (entity, mut pickup) in &mut pickups {
        let mut next = pickup.clone();
        match &mut next.origin {
            PickupOrigin::Spot {
                respawn_ticks_left, ..
            } => *respawn_ticks_left = respawn_ticks_left.saturating_sub(1),
            PickupOrigin::Drop { ticks_left } => {
                *ticks_left = ticks_left.saturating_sub(1);
                if *ticks_left == 0 && client.0.is_none() {
                    commands.entity(entity).insert(RemoveNetEntity);
                }
            }
        }
        pickup.set_if_neq(next);
    }
}

/// What a player with `inventory` would get from `contents`, and what would stay.
fn share(
    inventory: &mut Inventory,
    contents: &[ItemStack],
    catalog: &ItemCatalog,
) -> (Vec<ItemStack>, Vec<ItemStack>) {
    let (mut taken, mut left) = (Vec::new(), Vec::new());
    for stack in contents {
        let added = catalog
            .get(&stack.item)
            .map_or(0, |item| inventory.add(item, stack.count));
        let part = |count| ItemStack {
            item: stack.item.clone(),
            count,
        };
        if added > 0 {
            taken.push(part(added));
        }
        if added < stack.count {
            left.push(part(stack.count - added));
        }
    }
    (taken, left)
}

type Takers<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        &'static Transform,
        &'static Inventory,
        Option<&'static Health>,
    ),
    Without<RetiredEntity>,
>;

type Pickups<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Name, &'static NetPose, &'static mut Pickup),
    (Without<RetiredEntity>, Without<RemoveNetEntity>),
>;

/// What taking needs to know about the game.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Rules<'w, 's> {
    config: Res<'w, LootConfig>,
    catalog: Res<'w, ItemCatalog>,
    size: Res<'w, CharacterSize>,
    time: Res<'w, Time<Fixed>>,
    client: Res<'w, NetLocalClient>,
    matches: Query<'w, 's, &'static MatchState, Without<RetiredEntity>>,
}

pub(crate) fn take_pickups(
    players: Takers,
    // A player that respawns this tick is alive but still lies where it died (it is moved at the
    // end of the tick): it takes nothing there.
    mut respawns: MessageReader<PlayerRespawned>,
    mut pickups: Pickups,
    rules: Rules,
    mut grants: MessageWriter<GrantRequested>,
    mut commands: Commands,
) {
    let respawned: Vec<PlayerId> = respawns.read().map(|respawn| respawn.player).collect();
    if rules.client.0.is_some() || !rules.matches.iter().all(MatchState::allows_combat) {
        return;
    }
    // What each living player will carry, counting what it takes in this tick.
    let mut takers: Vec<(PlayerId, Vec3, Inventory)> = players
        .iter()
        .filter(|(player, .., health)| {
            health.is_none_or(Health::is_alive) && !respawned.contains(player)
        })
        .map(|(player, transform, inventory, _)| {
            let feet = rules.size.feet(transform.translation);
            (*player, feet, inventory.clone())
        })
        .collect();
    takers.sort_by_key(|(player, ..)| *player);
    let mut order: Vec<(String, Entity)> = pickups
        .iter()
        .filter(|(.., pickup)| pickup.is_there())
        .map(|(entity, name, ..)| (name.to_string(), entity))
        .collect();
    order.sort();
    for (_, entity) in order {
        let Ok((_, _, pose, mut pickup)) = pickups.get_mut(entity) else {
            continue;
        };
        let ground = pose.translation - Vec3::Y * HOVER_HEIGHT;
        for (player, feet, inventory) in &mut takers {
            if feet.xz().distance(ground.xz()) > rules.config.reach
                || (feet.y - ground.y).abs() > LEVEL_TOLERANCE
            {
                continue;
            }
            let (taken, left) = share(inventory, &pickup.contents, &rules.catalog);
            if taken.is_empty() {
                continue;
            }
            for stack in taken {
                grants.write(GrantRequested {
                    player: *player,
                    item: stack.item,
                    count: stack.count,
                });
            }
            let mut next = pickup.clone();
            match &mut next.origin {
                PickupOrigin::Spot {
                    kind,
                    respawn_ticks_left,
                } => {
                    let seconds = rules
                        .config
                        .kind(kind)
                        .map_or(0.0, |kind| kind.respawn_seconds);
                    *respawn_ticks_left = ticks_from_seconds(seconds, rules.time.timestep()).max(1);
                }
                PickupOrigin::Drop { .. } => {
                    next.contents = left;
                    if next.contents.is_empty() {
                        commands.entity(entity).insert(RemoveNetEntity);
                    }
                }
            }
            pickup.set_if_neq(next);
            break;
        }
    }
}

/// What the dead lost lies where they died.
pub(crate) fn drop_lost_items(
    mut lost: MessageReader<ItemsLost>,
    config: Res<LootConfig>,
    size: Res<CharacterSize>,
    time: Res<Time<Fixed>>,
    client: Res<NetLocalClient>,
    // Makes every drop's name unique for the whole session.
    mut dropped: Local<u32>,
    mut commands: Commands,
) {
    for loss in lost.read() {
        let ticks_left = ticks_from_seconds(config.drop_seconds, time.timestep());
        if client.0.is_some() || ticks_left == 0 {
            continue;
        }
        commands.spawn((
            Name::new(format!("drop/{}/{}", loss.player.0, *dropped)),
            Pickup {
                contents: loss.items.clone(),
                origin: PickupOrigin::Drop { ticks_left },
            },
            NetPose::at(size.feet(loss.at) + Vec3::Y * HOVER_HEIGHT),
        ));
        *dropped += 1;
    }
}

/// A new match: every spot is filled again and the drops are gone.
pub(crate) fn reset_on_restart(
    mut restarts: MessageReader<MatchRestarted>,
    mut pickups: Query<(Entity, &mut Pickup), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    mut commands: Commands,
) {
    if restarts.read().count() == 0 || client.0.is_some() {
        return;
    }
    for (entity, mut pickup) in &mut pickups {
        let mut next = pickup.clone();
        match &mut next.origin {
            PickupOrigin::Spot {
                respawn_ticks_left, ..
            } => *respawn_ticks_left = 0,
            PickupOrigin::Drop { .. } => {
                commands.entity(entity).insert(RemoveNetEntity);
            }
        }
        pickup.set_if_neq(next);
    }
}
