//! The module's tuning numbers (metres, seconds, radians).

use bevy::prelude::*;
use retreat_gameplay_api::{SoundCue, validate_cues};

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MovementConfig {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    /// How fast the wanted speed is reached on the ground (per second).
    pub ground_acceleration: f32,
    /// How fast a character slows on the ground (per second).
    pub ground_friction: f32,
    /// Friction uses at least this speed, so slow slides stop.
    pub stop_speed: f32,
    pub air_acceleration: f32,
    /// Most speed air control can add towards the wanted direction.
    pub air_wish_speed: f32,
    pub gravity: f32,
    pub jump_height: f32,
    pub max_fall_speed: f32,
    pub dash_speed: f32,
    pub dash_seconds: f32,
    pub dash_cooldown_seconds: f32,
    /// Radians of view per count of mouse movement.
    pub mouse_sensitivity: f32,
    /// Furthest a player can look up or down.
    pub max_pitch: f32,
    /// Highest step climbed and walked down without leaving the ground.
    pub step_height: f32,
    /// Steepest ground a character walks on.
    pub max_slope_degrees: f32,
    /// Heard from a character that jumps.
    pub jump_sound: Option<SoundCue>,
    /// Heard from a character that dashes.
    pub dash_sound: Option<SoundCue>,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            walk_speed: 6.0,
            sprint_speed: 9.0,
            ground_acceleration: 10.0,
            ground_friction: 6.0,
            stop_speed: 1.0,
            air_acceleration: 10.0,
            air_wish_speed: 1.0,
            gravity: 19.6,
            jump_height: 1.5,
            max_fall_speed: 25.0,
            dash_speed: 18.0,
            dash_seconds: 0.15,
            dash_cooldown_seconds: 3.0,
            mouse_sensitivity: 0.0022,
            max_pitch: 85.0_f32.to_radians(),
            step_height: 0.35,
            max_slope_degrees: 45.0,
            jump_sound: None,
            dash_sound: None,
        }
    }
}

impl MovementConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("walk_speed", self.walk_speed),
            ("sprint_speed", self.sprint_speed),
            ("ground_acceleration", self.ground_acceleration),
            ("ground_friction", self.ground_friction),
            ("stop_speed", self.stop_speed),
            ("air_acceleration", self.air_acceleration),
            ("air_wish_speed", self.air_wish_speed),
            ("gravity", self.gravity),
            ("jump_height", self.jump_height),
            ("max_fall_speed", self.max_fall_speed),
            ("dash_speed", self.dash_speed),
            ("dash_seconds", self.dash_seconds),
            ("dash_cooldown_seconds", self.dash_cooldown_seconds),
            ("mouse_sensitivity", self.mouse_sensitivity),
            ("max_pitch", self.max_pitch),
            ("step_height", self.step_height),
            ("max_slope_degrees", self.max_slope_degrees),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                return Err(format!("{field} must be 0 or more, got {value}"));
            }
        }
        if self.max_slope_degrees >= 90.0 {
            return Err("max_slope_degrees must be below 90".to_string());
        }
        if self.max_pitch >= std::f32::consts::FRAC_PI_2 {
            return Err("max_pitch must be below a quarter turn".to_string());
        }
        validate_cues([&self.jump_sound, &self.dash_sound])
    }
}
