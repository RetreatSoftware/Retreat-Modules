//! The end of a match and the start of the next, heard by everyone.

use crate::config::MatchConfig;
use bevy::prelude::*;
use retreat_engine_api::{EffectTime, NetState, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{MatchPhase, MatchState};
use serde::{Deserialize, Serialize};

/// On the match entity: counts the matches, so each start and end has its own sound id.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchNumber(pub u32);

impl NetState for MatchNumber {
    const ID: &'static str = "match.number";
}

/// Plays when the match or its phase changes from what this machine saw last. The first sight is
/// silent (a player may join in the middle).
pub(crate) fn play_match_sounds(
    matches: Query<(&MatchState, &MatchNumber), Without<RetiredEntity>>,
    config: Res<MatchConfig>,
    mut seen: Local<Option<(u32, bool)>>,
    mut sounds: ResMut<SoundEvents>,
) {
    for (state, number) in &matches {
        let finished = matches!(state.phase, MatchPhase::Finished { .. });
        let now = (number.0, finished);
        let before = seen.replace(now);
        if before.is_none_or(|before| before == now) {
            continue;
        }
        let (cue, what) = if finished {
            (&config.end_sound, "end")
        } else {
            (&config.start_sound, "start")
        };
        if let Some(cue) = cue {
            let id = format!("match/{}/{what}", number.0);
            sounds.play(cue.event(id, None, EffectTime::Present));
        }
    }
}
