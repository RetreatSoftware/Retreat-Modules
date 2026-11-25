//! Aiming: the camera looks over a shoulder, the shot leaves the eyes. This machine finds what the
//! center of the screen shows and aims the eyes at it.

use bevy::math::ops;
use bevy::prelude::*;
use retreat_gameplay_api::LookAngles;

/// The view angles from `eye` towards `target`; `None` when they are the same point.
pub fn aim_angles(eye: Vec3, target: Vec3) -> Option<LookAngles> {
    let direction = (target - eye).try_normalize()?;
    Some(LookAngles {
        yaw: ops::atan2(-direction.x, -direction.z),
        pitch: ops::asin(direction.y.clamp(-1.0, 1.0)),
    })
}
