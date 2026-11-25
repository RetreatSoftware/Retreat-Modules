//! Where players may appear.

use bevy::prelude::*;

/// A place a character may appear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnPoint {
    /// Where the character's feet go, in metres, Y up.
    pub feet: Vec3,
    /// Direction the character looks: rotation about +Y in radians, 0 looks along -Z.
    pub yaw: f32,
}

/// The loaded map's spawn points, in a fixed order. Empty until a map is loaded.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SpawnPoints(pub Vec<SpawnPoint>);
