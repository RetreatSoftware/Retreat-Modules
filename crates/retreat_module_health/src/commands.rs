//! `health.*` and `assert.health*`. None is allowed for clients: only the server and scripts run them.

use crate::damage::KillRequested;
use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandOutcome, CommandResult, NetState, RetiredEntity};
use retreat_gameplay_api::{
    DamageCause, DamageRequested, Health, MAX_PLAYERS, PlayerId, PlayerRespawned, SpawnPoints,
    character_center, character_feet, choose_spawn_point, player_entity_name,
};
use serde::{Deserialize, Serialize};

/// A standing target made by `health.add_dummy`: a player with no body and no controls, for scripts
/// and for trying weapons. Real characters come from the module that spawns players.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthDummy;

impl NetState for HealthDummy {
    const ID: &'static str = "health.dummy";
}

type Dummies = (With<HealthDummy>, Without<RetiredEntity>);

pub(crate) fn add_dummy_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let player = PlayerId(args.parse(0, "player")?);
    let name = player_entity_name(player);
    let mut existing = world.query::<(&Name, Option<&Transform>, Has<PlayerId>)>();
    let mut occupied = Vec::new();
    let mut players = 0;
    for (found, transform, is_player) in existing.iter(world) {
        if found.as_str() == name {
            return Err(args.failure(format!("an entity named `{name}` already exists")));
        }
        if is_player {
            players += 1;
            occupied.extend(transform.map(|t| character_feet(t.translation)));
        }
    }
    if players >= MAX_PLAYERS {
        return Err(args.failure(format!("there are already {MAX_PLAYERS} players")));
    }
    let points = &world.resource::<SpawnPoints>().0;
    let feet = choose_spawn_point(points, &occupied).map_or(Vec3::ZERO, |index| points[index].feet);
    world.spawn((
        Name::new(name),
        player,
        HealthDummy,
        Transform::from_translation(character_center(feet)),
    ));
    Ok(CommandOutcome::Continue)
}

/// Dummies have no owner that would move them, so this module does.
pub(crate) fn move_dummies_to_spawn(
    mut respawned: MessageReader<PlayerRespawned>,
    mut dummies: Query<(&PlayerId, &mut Transform), Dummies>,
) {
    for message in respawned.read() {
        for (player, mut transform) in &mut dummies {
            if *player == message.player {
                transform.translation = character_center(message.spawn.feet);
            }
        }
    }
}

pub(crate) fn damage_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let has_attacker = !args.rest(2).is_empty();
    args.expect_count(if has_attacker { 3 } else { 2 })?;
    let target = PlayerId(args.parse(0, "player")?);
    let amount: u32 = args.parse(1, "amount")?;
    let attacker = if has_attacker {
        Some(PlayerId(args.parse(2, "attacker")?))
    } else {
        None
    };
    health_of(world, args, target)?;
    world.write_message(DamageRequested {
        target,
        amount,
        attacker,
        cause: match attacker {
            Some(_) => DamageCause::Hitscan,
            None => DamageCause::World,
        },
    });
    Ok(CommandOutcome::Continue)
}

pub(crate) fn kill_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let target = PlayerId(args.parse(0, "player")?);
    health_of(world, args, target)?;
    world.write_message(KillRequested(target));
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_health_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: u32 = args.parse(1, "hit points")?;
    let actual = health_of(world, args, player)?.current();
    if actual != expected {
        return Err(args.failure(format!(
            "player {} has {actual} hit points, expected {expected}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_dead_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let player = PlayerId(args.parse(0, "player")?);
    let health = health_of(world, args, player)?;
    if health.is_alive() {
        return Err(args.failure(format!(
            "player {} is alive with {} hit points",
            player.0,
            health.current()
        )));
    }
    Ok(CommandOutcome::Continue)
}

fn health_of(
    world: &mut World,
    args: &CommandArgs,
    player: PlayerId,
) -> Result<Health, retreat_engine_api::CommandError> {
    let mut players = world.query_filtered::<(&PlayerId, &Health), Without<RetiredEntity>>();
    players
        .iter(world)
        .find(|(found, _)| **found == player)
        .map(|(_, health)| *health)
        .ok_or_else(|| {
            args.failure(format!(
                "there is no player {} with health (a player gets it in the tick it appears)",
                player.0
            ))
        })
}
