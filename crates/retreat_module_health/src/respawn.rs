//! Getting health, protection and respawn timers, and coming back to life.

use crate::config::HealthConfig;
use crate::state::{RespawnTimer, SpawnProtection};
use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{
    Health, MatchRestarted, PlayerId, PlayerRespawned, SpawnPoint, SpawnPoints, character_feet,
    choose_spawn_point, ticks_from_seconds,
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
            .insert(Health::full(config.max_health));
        protect(&mut commands, entity, &config, &time);
    }
}

/// Counts protection and respawn timers down by one tick; respawns whose delay ran out.
pub(crate) fn advance_timers(
    mut players: Players,
    config: Res<HealthConfig>,
    time: Res<Time<Fixed>>,
    points: Res<SpawnPoints>,
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
        .filter_map(|(_, _, _, transform, ..)| transform.map(|t| character_feet(t.translation)))
        .collect();
    // By player id: query order can change after a rewind.
    due.sort();
    for (_, entity) in due {
        if let Some(message) = respawn(&mut players, entity, &points, &mut occupied) {
            respawned.write(message);
            protect(&mut commands, entity, &config, &time);
            commands.entity(entity).remove::<RespawnTimer>();
        }
    }
}

/// A new match: everyone, dead or alive, starts again.
pub(crate) fn respawn_all_on_restart(
    mut restarts: MessageReader<MatchRestarted>,
    mut players: Players,
    config: Res<HealthConfig>,
    time: Res<Time<Fixed>>,
    points: Res<SpawnPoints>,
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
        if let Some(message) = respawn(&mut players, entity, &points, &mut occupied) {
            respawned.write(message);
            commands.entity(entity).remove::<RespawnTimer>();
            protect(&mut commands, entity, &config, &time);
        }
    }
}

/// Restores health and picks the spawn point, adding it to `occupied`.
fn respawn(
    players: &mut Players,
    entity: Entity,
    points: &SpawnPoints,
    occupied: &mut Vec<Vec3>,
) -> Option<PlayerRespawned> {
    let (_, player, mut health, transform, ..) = players.get_mut(entity).ok()?;
    health.restore();
    let spawn = match choose_spawn_point(&points.0, occupied) {
        Some(index) => points.0[index],
        None => SpawnPoint {
            feet: transform.map_or(Vec3::ZERO, |t| character_feet(t.translation)),
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
