//! Where the camera wants to be, without physics.

use crate::config::ShoulderConfig;
use bevy::prelude::*;
use retreat_gameplay_api::{LookAngles, character_feet, horizontal_axes, view_direction};

/// The three points of an over-the-shoulder view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderPoints {
    /// Inside the character's head: the cast starts here.
    pub pivot: Vec3,
    /// Beside the head.
    pub shoulder: Vec3,
    /// Behind the shoulder, when nothing is in the way.
    pub camera: Vec3,
}

/// `side` is 1 for the right shoulder and -1 for the left.
pub fn shoulder_points(
    center: Vec3,
    look: LookAngles,
    side: f32,
    config: &ShoulderConfig,
) -> ShoulderPoints {
    let pivot = character_feet(center) + Vec3::Y * config.pivot_height;
    let (_, right) = horizontal_axes(look.yaw);
    let shoulder = pivot + right * (config.shoulder_offset * side);
    ShoulderPoints {
        pivot,
        shoulder,
        camera: shoulder - view_direction(look) * config.distance,
    }
}

/// 0 when the camera is at least `close_distance` behind the shoulder, 1 when it is right at it.
pub fn closeness(behind: f32, config: &ShoulderConfig) -> f32 {
    (1.0 - behind / config.close_distance).clamp(0.0, 1.0)
}

/// The view tilted down by what `closeness` asks, never past straight down.
pub fn tilted(look: LookAngles, closeness: f32, config: &ShoulderConfig) -> LookAngles {
    const STEEPEST: f32 = -1.5;
    LookAngles {
        yaw: look.yaw,
        pitch: (look.pitch - config.close_tilt * closeness).max(STEEPEST),
    }
}
