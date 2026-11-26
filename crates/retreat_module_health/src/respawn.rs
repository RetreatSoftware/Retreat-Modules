//! Getting health, protection and respawn timers, and coming back to life.

use crate::config::HealthConfig;
use crate::state::{Life, SpawnProtection};
use bevy::prelude::*;
use retreat_engine_api::{Intangible, RetiredEntity};
use retreat_gameplay_api::{
    CharacterSize, Health, MatchRestarted, PlayerFired, PlayerId, PlayerRespawned, RespawnTimer,
    SpawnPoint, SpawnPoints, choose_spawn_point, ticks_from_seconds,
};

type Players<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        &'static mut Health,
        Option<&'static Transform>,
        Option<&'static mut RespawnTimer>,
        Option<&'static mut SpawnProtection>,
    ),
    Without<RetiredEntity>,
>;

/// Where a player can respawn, and how big it is there.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Places<'w> {
    points: Res<'w, SpawnPoints>,
    size: Res<'w, CharacterSize>,
}

type NewPlayers = (With<PlayerId>, Without<Health>, Without<RetiredEntity>);

/// Gives every new player full health and spawn protection.
pub(crate) fn attach_health(
    new_players: Query<Entity, NewPlayers>,
    config: Res<HealthConfig>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    for entity in &new_players {
        commands
            .entity(entity)
            .insert((Health::full(config.max_health), Life::default()));
        protect(&mut commands, entity, &config, &time);
    }
}

type Protected = (With<SpawnProtection>, Without<RetiredEntity>);

/// Whoever shoots is no longer protected.
pub(crate) fn end_protection_on_fire(
    mut fired: MessageReader<PlayerFired>,
    protected: Query<(Entity, &PlayerId), Protected>,
    mut commands: Commands,
) {
    for shot in fired.read() {
        for (entity, player) in &protected {
            if *player == shot.player {
                commands.entity(entity).remove::<SpawnProtection>();
            }
        }
    }
}

/// Counts protection and respawn timers down by one tick; respawns whose delay ran out.
pub(crate) fn advance_timers(
    mut players: Players,
    config: Res<HealthConfig>,
    time: Res<Time<Fixed>>,
    places: Places,
    mut respawned: MessageWriter<PlayerRespawned>,
    mut commands: Commands,
) {
    let mut due: Vec<(PlayerId, Entity)> = Vec::new();
    for (entity, player, _, _, respawn, protection) in &mut players {
        if let Some(mut protection) = protection {
            protection.ticks_left = protection.ticks_left.saturating_sub(1);
            if protection.ticks_left == 0 {
                commands.entity(entity).remove::<SpawnProtection>();
            }
        }
        if let Some(mut respawn) = respawn {
            respawn.ticks_left = respawn.ticks_left.saturating_sub(1);
            if respawn.ticks_left == 0 {
                due.push((*player, entity));
            }
        }
    }
    if due.is_empty() {
        return;
    }
    let mut occupied: Vec<Vec3> = players
        .iter()
        .filter(|(_, _, health, ..)| health.is_alive())
        .filter_map(|(_, _, _, transform, ..)| transform.map(|t| places.size.feet(t.translation)))
        .collect();
    // By player id: query order can change after a rewind.
    due.sort();
    for (_, entity) in due {
        if let Some(message) = respawn(&mut players, entity, &places, &mut occupied) {
            respawned.write(message);
            protect(&mut commands, entity, &config, &time);
            commands
                .entity(entity)
                .remove::<(RespawnTimer, Intangible)>();
        }
    }
}

/// A new match: everyone, dead or alive, starts again.
pub(crate) fn respawn_all_on_restart(
    mut restarts: MessageReader<MatchRestarted>,
    mut players: Players,
    config: Res<HealthConfig>,
    time: Res<Time<Fixed>>,
    places: Places,
    mut respawned: MessageWriter<PlayerRespawned>,
    mut commands: Commands,
) {
    if restarts.read().count() == 0 {
        return;
    }
    let mut everyone: Vec<(PlayerId, Entity)> = players
        .iter()
        .map(|(entity, player, ..)| (*player, entity))
        .collect();
    everyone.sort();
    let mut occupied = Vec::new();
    for (_, entity) in everyone {
        if let Some(message) = respawn(&mut players, entity, &places, &mut occupied) {
            respawned.write(message);
            commands
                .entity(entity)
                .remove::<(RespawnTimer, Intangible)>();
            protect(&mut commands, entity, &config, &time);
        }
    }
}

/// Restores health and picks the spawn point, adding it to `occupied`.
fn respawn(
    players: &mut Players,
    entity: Entity,
    places: &Places,
    occupied: &mut Vec<Vec3>,
) -> Option<PlayerRespawned> {
    let (_, player, mut health, transform, ..) = players.get_mut(entity).ok()?;
    health.restore();
    let spawn = match choose_spawn_point(&places.points.0, occupied) {
        Some(index) => places.points.0[index],
        None => SpawnPoint {
            feet: transform.map_or(Vec3::ZERO, |t| places.size.feet(t.translation)),
            yaw: 0.0,
        },
    };
    occupied.push(spawn.feet);
    Some(PlayerRespawned {
        player: *player,
        spawn,
    })
}

fn protect(commands: &mut Commands, entity: Entity, config: &HealthConfig, time: &Time<Fixed>) {
    let ticks_left = ticks_from_seconds(config.spawn_protection_seconds, time.timestep());
    if ticks_left > 0 {
        commands
            .entity(entity)
            .insert(SpawnProtection { ticks_left });
    }
}
