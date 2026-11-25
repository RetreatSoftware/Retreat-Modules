//! The module's numbers.

use bevy::prelude::*;
use retreat_gameplay_api::CHARACTER_HEIGHT;

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ShoulderConfig {
    /// Height above the feet the view turns around.
    pub pivot_height: f32,
    /// Distance behind the shoulder.
    pub distance: f32,
    /// Sideways offset of the shoulder.
    pub shoulder_offset: f32,
    pub fov_degrees: f32,
    /// How close the camera may get to geometry.
    pub wall_margin: f32,
    /// A camera pushed closer than this behind the shoulder moves over the character and looks
    /// down, the more the closer, so the character stays in the picture.
    pub close_distance: f32,
    /// How far it rises when it is right behind the character, metres.
    pub close_lift: f32,
    /// How far it then tilts down, radians.
    pub close_tilt: f32,
}

impl Default for ShoulderConfig {
    fn default() -> Self {
        Self {
            pivot_height: 1.5,
            distance: 3.5,
            shoulder_offset: 0.6,
            fov_degrees: 70.0,
            wall_margin: 0.2,
            close_distance: 2.0,
            close_lift: 0.7,
            close_tilt: 0.8,
        }
    }
}

impl ShoulderConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("distance", self.distance),
            ("wall_margin", self.wall_margin),
            ("close_distance", self.close_distance),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("{field} must be more than 0, got {value}"));
            }
        }
        if !(0.0..=CHARACTER_HEIGHT).contains(&self.pivot_height) {
            return Err(format!(
                "pivot_height must be within the character (0 to {CHARACTER_HEIGHT})"
            ));
        }
        if !(self.shoulder_offset.is_finite() && self.shoulder_offset >= 0.0) {
            return Err("shoulder_offset must be 0 or more".to_string());
        }
        for (field, value) in [
            ("close_lift", self.close_lift),
            ("close_tilt", self.close_tilt),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                return Err(format!("{field} must be 0 or more, got {value}"));
            }
        }
        if !(1.0..=179.0).contains(&self.fov_degrees) {
            return Err("fov_degrees must be from 1 to 179".to_string());
        }
        Ok(())
    }
}
