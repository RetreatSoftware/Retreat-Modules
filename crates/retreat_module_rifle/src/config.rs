//! The module's tuning numbers.

use bevy::prelude::*;
use retreat_gameplay_api::{SoundCue, validate_cues};

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct RifleConfig {
    /// Hit points one hit removes.
    pub damage: u32,
    /// Time between two shots of automatic fire.
    pub fire_interval_seconds: f32,
    /// Rounds in a full magazine; the reserve is unlimited.
    pub magazine: u32,
    /// Time a reload takes. It starts by itself when the magazine is empty.
    pub reload_seconds: f32,
    /// How far a shot reaches, metres.
    pub range: f32,
    /// Heard from the shooter at every shot.
    pub shot_sound: Option<SoundCue>,
    /// Heard by the reloading player.
    pub reload_sound: Option<SoundCue>,
}

impl Default for RifleConfig {
    fn default() -> Self {
        Self {
            damage: 20,
            fire_interval_seconds: 0.15,
            magazine: 30,
            reload_seconds: 1.5,
            range: 100.0,
            shot_sound: None,
            reload_sound: None,
        }
    }
}

impl RifleConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if self.magazine == 0 {
            return Err("magazine must be at least 1".to_string());
        }
        for (field, value) in [
            ("fire_interval_seconds", self.fire_interval_seconds),
            ("reload_seconds", self.reload_seconds),
            ("range", self.range),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("{field} must be more than 0, got {value}"));
            }
        }
        validate_cues([&self.shot_sound, &self.reload_sound])
    }
}
