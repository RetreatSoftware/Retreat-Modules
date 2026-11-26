//! The module's tuning numbers.

use bevy::prelude::*;
use retreat_gameplay_api::{SoundCue, validate_cues};

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MatchConfig {
    pub length_seconds: f32,
    /// How long the result is shown before the next match starts.
    pub result_seconds: f32,
    pub points_per_kill: i32,
    /// Added on a self-kill or a world death; normally negative.
    pub points_per_self_kill: i32,
    /// Heard by everyone when a new match starts.
    pub start_sound: Option<SoundCue>,
    /// Heard by everyone when the match is over.
    pub end_sound: Option<SoundCue>,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            length_seconds: 300.0,
            result_seconds: 10.0,
            points_per_kill: 1,
            points_per_self_kill: -1,
            start_sound: None,
            end_sound: None,
        }
    }
}

impl MatchConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        for (field, seconds) in [
            ("length_seconds", self.length_seconds),
            ("result_seconds", self.result_seconds),
        ] {
            if !(seconds.is_finite() && seconds > 0.0) {
                return Err(format!("{field} must be more than 0, got {seconds}"));
            }
        }
        validate_cues([&self.start_sound, &self.end_sound])
    }
}
