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

/// A place of the map where something to pick up appears.
#[derive(Debug, Clone, PartialEq)]
pub struct PickupSpot {
    /// Unique in the map; lowercase, digits and `_`.
    pub name: String,
    /// What belongs here (`health`, `weapon`): the game says what each kind holds.
    pub kind: String,
    /// The ground under it, in metres, Y up.
    pub feet: Vec3,
}

/// The loaded map's pickup spots, in a fixed order. Empty until a map is loaded.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct PickupSpots(pub Vec<PickupSpot>);
