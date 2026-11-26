//! The module's tuning numbers.

use bevy::prelude::*;
use retreat_gameplay_api::{SoundCue, validate_cues};

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HealthConfig {
    pub max_health: u32,
    /// Time between death and respawn.
    pub respawn_delay_seconds: f32,
    /// Time after a (re)spawn in which damage is ignored; 0 turns it off.
    pub spawn_protection_seconds: f32,
    /// A living player below this height dies.
    pub fall_out_height: f32,
    /// Heard from a player that dies.
    pub death_sound: Option<SoundCue>,
    /// Heard from a player that comes back.
    pub respawn_sound: Option<SoundCue>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_health: 100,
            respawn_delay_seconds: 3.0,
            spawn_protection_seconds: 1.0,
            fall_out_height: -10.0,
            death_sound: None,
            respawn_sound: None,
        }
    }
}

impl HealthConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_health == 0 {
            return Err("max_health must be at least 1".to_string());
        }
        for (field, seconds) in [
            ("respawn_delay_seconds", self.respawn_delay_seconds),
            ("spawn_protection_seconds", self.spawn_protection_seconds),
        ] {
            if !(seconds.is_finite() && seconds >= 0.0) {
                return Err(format!("{field} must be 0 or more, got {seconds}"));
            }
        }
        if !self.fall_out_height.is_finite() {
            return Err("fall_out_height must be a finite number".to_string());
        }
        validate_cues([&self.death_sound, &self.respawn_sound])
    }
}
