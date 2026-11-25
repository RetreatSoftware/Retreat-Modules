//! Converts positions from the source of the graybox layout (the user's NetworkStudy project) into
//! Retreat's units.
//!
//! Source: centimetres, Z up, +X forward, +Y right. Retreat: metres, Y up, -Z forward, +X right.
//! So the layout looks the same to a player in both (not mirrored).

use bevy::prelude::*;

const CENTIMETRES_PER_METRE: f32 = 100.0;

pub fn source_cm_to_retreat(source: Vec3) -> Vec3 {
    Vec3::new(source.y, source.z, -source.x) / CENTIMETRES_PER_METRE
}

pub fn retreat_to_source_cm(position: Vec3) -> Vec3 {
    Vec3::new(-position.z, position.x, position.y) * CENTIMETRES_PER_METRE
}
