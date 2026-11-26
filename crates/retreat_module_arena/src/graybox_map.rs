//! The graybox arena: 60 x 60 m floor, 4 m walls, a 1 m and a 2 m platform, a staircase, cover and
//! pillars. The box coordinates are the layout of the user's NetworkStudy arena, kept in its units
//! (centimetres, Z up) so they can be compared with it, and converted by `source_cm_to_retreat`.
//! The spawn points are Retreat's own, in metres.

use crate::map::{ArenaMap, MapPiece, PieceKind};
use crate::source_units::source_cm_to_retreat;
use bevy::math::ops;
use bevy::prelude::*;
use retreat_gameplay_api::{PickupSpot, SpawnPoint};

/// `(name, kind, corner, opposite corner)` in source centimetres, Z up.
type SourceBox = (&'static str, PieceKind, [f32; 3], [f32; 3]);

use PieceKind::{Cover, Floor, Pillar, Platform, Wall};

#[rustfmt::skip]
const BOXES: &[SourceBox] = &[
    // The floor is thick so nothing tunnels through it; its top is height 0.
    ("floor", Floor, [-3000.0, -3000.0, -200.0], [3000.0, 3000.0, 0.0]),
    // Perimeter walls (named by the side as seen from the arena center looking along -Z).
    ("wall_south", Wall, [-3100.0, -3100.0, 0.0], [-3000.0, 3100.0, 400.0]),
    ("wall_north", Wall, [3000.0, -3100.0, 0.0], [3100.0, 3100.0, 400.0]),
    ("wall_west", Wall, [-3000.0, -3100.0, 0.0], [3000.0, -3000.0, 400.0]),
    ("wall_east", Wall, [-3000.0, 3000.0, 0.0], [3000.0, 3100.0, 400.0]),
    // 1 m platform, and the 2 m platform reached by jumping from it.
    ("platform_low", Platform, [800.0, -500.0, 0.0], [1600.0, 500.0, 100.0]),
    ("platform_high", Platform, [1600.0, -500.0, 0.0], [2300.0, 500.0, 200.0]),
    // Cover blocks.
    ("cover_1", Cover, [-900.0, -300.0, 0.0], [-700.0, -100.0, 100.0]),
    ("cover_2", Cover, [-900.0, 300.0, 0.0], [-700.0, 500.0, 200.0]),
    ("cover_3", Cover, [-1600.0, 0.0, 0.0], [-1200.0, 200.0, 120.0]),
    ("cover_4", Cover, [-400.0, 1200.0, 0.0], [0.0, 1400.0, 100.0]),
    ("cover_5", Cover, [300.0, -1500.0, 0.0], [700.0, -1300.0, 100.0]),
    ("cover_6", Cover, [-2200.0, -1800.0, 0.0], [-1800.0, -1400.0, 200.0]),
    ("cover_7", Cover, [1200.0, 1500.0, 0.0], [1400.0, 1700.0, 150.0]),
    // Pillars.
    ("pillar_1", Pillar, [-300.0, -900.0, 0.0], [-200.0, -800.0, 400.0]),
    ("pillar_2", Pillar, [300.0, 900.0, 0.0], [400.0, 1000.0, 400.0]),
    ("pillar_3", Pillar, [-1500.0, 1500.0, 0.0], [-1400.0, 1600.0, 400.0]),
];

/// The staircase up to the low platform: each step starts further along and is higher, and all end
/// where the platform begins. 10 cm risers, so a character needs a step height of at least 0.1 m.
const STAIR_STEPS: u8 = 10;
const STAIR_START_X: f32 = 200.0;
const STAIR_END_X: f32 = 800.0;
const STAIR_HALF_WIDTH: f32 = 150.0;
const STAIR_RISE: f32 = 10.0;

/// Spawn points on the floor near the walls, `(x, z)` in metres, all facing the center.
const SPAWN_XZ: &[(f32, f32)] = &[
    (0.0, 26.0),
    (0.0, -26.0),
    (26.0, 0.0),
    (-26.0, 0.0),
    (24.0, 24.0),
    (-24.0, -24.0),
    (24.0, -24.0),
    (-24.0, 24.0),
];

/// Pickup spots `(name, kind, [x, feet y, z])` in metres: health and ammunition around the floor,
/// rockets east and west, the launcher on the high platform.
const PICKUP_SPOTS: &[(&str, &str, [f32; 3])] = &[
    ("health_west", "health", [-20.0, 0.0, -6.0]),
    ("health_east", "health", [20.0, 0.0, 6.0]),
    ("health_north", "health", [10.0, 0.0, -10.0]),
    ("health_south", "health", [-12.0, 0.0, 12.0]),
    ("ammo_southeast", "ammo", [14.0, 0.0, 18.0]),
    ("ammo_northwest", "ammo", [-14.0, 0.0, -14.0]),
    ("ammo_northeast", "ammo", [20.0, 0.0, -20.0]),
    ("ammo_southwest", "ammo", [-22.0, 0.0, 18.0]),
    ("rockets_east", "rockets", [22.0, 0.0, -4.0]),
    ("rockets_west", "rockets", [-22.0, 0.0, 4.0]),
    ("launcher_platform", "launcher", [-3.5, 2.0, -21.5]),
];

pub fn graybox_map() -> ArenaMap {
    let mut pieces: Vec<MapPiece> = BOXES
        .iter()
        .map(|&(name, kind, from, to)| piece(name.to_string(), kind, from.into(), to.into()))
        .collect();
    let tread = (STAIR_END_X - STAIR_START_X) / f32::from(STAIR_STEPS);
    pieces.extend((0..STAIR_STEPS).map(|step| {
        let x = STAIR_START_X + f32::from(step) * tread;
        let height = f32::from(step + 1) * STAIR_RISE;
        piece(
            format!("stair_{}", step + 1),
            PieceKind::Stair,
            Vec3::new(x, -STAIR_HALF_WIDTH, 0.0),
            Vec3::new(STAIR_END_X, STAIR_HALF_WIDTH, height),
        )
    }));
    let spawn_points = SPAWN_XZ
        .iter()
        .map(|&(x, z)| SpawnPoint {
            feet: Vec3::new(x, 0.0, z),
            // Yaw 0 looks along -Z, so this looks from (x, z) at the center.
            yaw: ops::atan2(x, z),
        })
        .collect();
    ArenaMap {
        half_extent: 30.0,
        pieces,
        spawn_points,
        pickup_spots: PICKUP_SPOTS
            .iter()
            .map(|&(name, kind, feet)| PickupSpot {
                name: name.to_string(),
                kind: kind.to_string(),
                feet: feet.into(),
            })
            .collect(),
    }
}

/// A piece from two opposite corners in source units (the conversion flips an axis, so the corners
/// are re-sorted into min and max).
fn piece(name: String, kind: PieceKind, from: Vec3, to: Vec3) -> MapPiece {
    let (a, b) = (source_cm_to_retreat(from), source_cm_to_retreat(to));
    MapPiece {
        name,
        kind,
        min: a.min(b),
        max: a.max(b),
    }
}
