//! Death and respawn, heard from where they happen. Derived from synced state, so a client hears
//! the others too; the ids come from the player's life number.

use crate::config::HealthConfig;
use crate::state::{Life, SpawnProtection};
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{Health, PlayerId, PlayerRespawned, event_time};

/// Starts a new life for everyone who respawned in this tick.
pub(crate) fn count_lives(
    mut respawned: MessageReader<PlayerRespawned>,
    mut players: Query<(&PlayerId, &mut Life), Without<RetiredEntity>>,
) {
    for message in respawned.read() {
        for (player, mut life) in &mut players {
            if *player == message.player {
                life.0 += 1;
            }
        }
    }
}

type Living<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        &'static Health,
        &'static Life,
        &'static Transform,
        Has<SpawnProtection>,
    ),
    Without<RetiredEntity>,
>;

/// Runs every tick; a sound whose id was heard is not repeated. Death lasts as long as the respawn
/// delay and a new life's protection is short, both far below the engine's memory of ids.
pub(crate) fn play_life_sounds(
    players: Living,
    config: Res<HealthConfig>,
    client: Res<NetLocalClient>,
    mut sounds: ResMut<SoundEvents>,
) {
    for (player, health, life, transform, protected) in &players {
        let (cue, what) = if !health.is_alive() {
            (&config.death_sound, "death")
        } else if protected && life.0 > 0 {
            (&config.respawn_sound, "respawn")
        } else {
            continue;
        };
        if let Some(cue) = cue {
            let id = format!("{what}/{}/{}", player.0, life.0);
            let time = event_time(*client, *player);
            sounds.play(cue.event(id, Some(transform.translation), time));
        }
    }
}
