//! Jumps and dashes, heard from the character. Derived from the counts in the synced state: this
//! machine predicts its own and learns of the others' when their state arrives.

use crate::config::MovementConfig;
use crate::state::MovementState;
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{PlayerId, SoundCue, event_time};
use std::collections::BTreeMap;

/// `(jumps, dashes)` per player as last seen. The first sight is silent (it may be old).
pub(crate) fn play_movement_sounds(
    players: Query<(&PlayerId, &MovementState, &Transform), Without<RetiredEntity>>,
    config: Res<MovementConfig>,
    client: Res<NetLocalClient>,
    mut seen: Local<BTreeMap<PlayerId, (u32, u32)>>,
    mut sounds: ResMut<SoundEvents>,
) {
    for (player, state, transform) in &players {
        let now = (state.jumps, state.dashes);
        let Some(before) = seen.insert(*player, now) else {
            continue;
        };
        let mut play = |cue: &Option<SoundCue>, what: &str, count: u32| {
            if let Some(cue) = cue {
                let id = format!("move/{}/{what}/{count}", player.0);
                let time = event_time(*client, *player);
                sounds.play(cue.event(id, Some(transform.translation), time));
            }
        };
        if now.0 > before.0 {
            play(&config.jump_sound, "jump", now.0);
        }
        if now.1 > before.1 {
            play(&config.dash_sound, "dash", now.1);
        }
    }
}
