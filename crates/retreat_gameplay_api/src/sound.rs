//! The sounds gameplay modules play. A module's config names its sounds (assets belong to the game);
//! without one it is silent. These are the pure rules for turning a cue into the engine's one-shot
//! `SoundEvent`.

use crate::PlayerId;
use bevy::prelude::*;
use retreat_engine_api::{
    DEFAULT_SOUND_REACH, EffectTime, MAX_SOUND_VOLUME, NetLocalClient, SoundEvent, SoundPlayback,
    SoundPosition,
};

/// One sound of a game.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundCue {
    /// A file in the game's assets (`sounds/shot.wav`).
    pub path: String,
    /// Linear volume, 0 to `MAX_SOUND_VOLUME`.
    pub volume: f32,
}

impl SoundCue {
    pub fn new(path: &str, volume: f32) -> Self {
        Self {
            path: path.to_string(),
            volume,
        }
    }

    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        SoundPlayback {
            volume: self.volume,
            ..SoundPlayback::effect(self.path.clone())
        }
        .validate()
        .map_err(|problem| {
            format!(
                "sound `{}`: {problem} (at most {MAX_SOUND_VOLUME})",
                self.path
            )
        })
    }

    /// The event that plays this cue once under `id`, from `at` in the world (`None`: heard the
    /// same everywhere).
    pub fn event(&self, id: String, at: Option<Vec3>, time: EffectTime) -> SoundEvent {
        SoundEvent {
            id,
            path: self.path.clone(),
            volume: self.volume,
            at: at.map(|position| SoundPosition {
                position,
                reach: DEFAULT_SOUND_REACH,
            }),
            time,
        }
    }
}

/// Checks every cue that is set.
pub fn validate_cues<'a>(
    cues: impl IntoIterator<Item = &'a Option<SoundCue>>,
) -> Result<(), String> {
    cues.into_iter().flatten().try_for_each(SoundCue::validate)
}

/// When this machine shows or plays something `owner` did: at once, except that on a client another
/// player is drawn in the server's past, and its sounds and effects wait for that view.
pub fn event_time(client: NetLocalClient, owner: PlayerId) -> EffectTime {
    if client.0.is_some_and(|local| local != owner.0) {
        EffectTime::RemoteView
    } else {
        EffectTime::Present
    }
}
