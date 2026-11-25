//! Where a player looks.

use bevy::math::ops;
use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// A player's view angles in radians. Yaw turns about +Y (0 looks along -Z, positive turns left);
/// pitch is positive upwards.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LookAngles {
    pub yaw: f32,
    pub pitch: f32,
}

impl NetState for LookAngles {
    const ID: &'static str = "gameplay.look";
}

/// `(forward, right)` on the ground for a yaw, both unit length.
pub fn horizontal_axes(yaw: f32) -> (Vec3, Vec3) {
    // `ops`: the same bits on every platform, so predictions agree with the server.
    let (sin, cos) = ops::sin_cos(yaw);
    (Vec3::new(-sin, 0.0, -cos), Vec3::new(cos, 0.0, -sin))
}

/// The unit direction a player with these angles looks along.
pub fn view_direction(look: LookAngles) -> Vec3 {
    let (forward, _) = horizontal_axes(look.yaw);
    let (sin, cos) = ops::sin_cos(look.pitch);
    forward * cos + Vec3::Y * sin
}
