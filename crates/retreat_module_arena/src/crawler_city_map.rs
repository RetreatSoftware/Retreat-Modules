//! `crawler_city`: a **placeholder** graybox for the crawler (crawler plan, phase 1). Two areas of
//! one map, far apart: the safe city (80 x 80 m, walled, a plaza with a fountain, houses and market
//! stalls; new characters start on the plaza) and a small placeholder dungeon 500 m east (a walled
//! room with pillars), there only so characters can be moved between areas until the real dungeon
//! map (phase 3). Metres, Y up. Every shape, size and rule number here is a placeholder.

use bevy::math::ops;
use bevy::prelude::*;
use retreat_gameplay_api::{Area, AreaId, AreaRules, SpawnPoint};

use crate::map::{ArenaMap, MapPiece, PieceKind};

use PieceKind::{Cover, Floor, Pillar, Wall};

/// `(name, kind, min corner, max corner)` in metres.
type MetreBox = (&'static str, PieceKind, [f32; 3], [f32; 3]);

/// Half the city's width (the city spans -40..40 on x and z).
const CITY_HALF: f32 = 40.0;
/// Where the placeholder dungeon's middle is on x, and its half sizes.
const DUNGEON_X: f32 = 500.0;
const DUNGEON_HALF_X: f32 = 20.0;
const DUNGEON_HALF_Z: f32 = 12.0;
const WALL_HEIGHT: f32 = 4.0;

#[rustfmt::skip]
const CITY_BOXES: &[MetreBox] = &[
    // Thick floor whose top is height 0.
    ("city_floor", Floor, [-40.0, -2.0, -40.0], [40.0, 0.0, 40.0]),
    ("city_wall_north", Wall, [-41.0, 0.0, -41.0], [41.0, WALL_HEIGHT, -40.0]),
    ("city_wall_south", Wall, [-41.0, 0.0, 40.0], [41.0, WALL_HEIGHT, 41.0]),
    ("city_wall_west", Wall, [-41.0, 0.0, -40.0], [-40.0, WALL_HEIGHT, 40.0]),
    ("city_wall_east", Wall, [40.0, 0.0, -40.0], [41.0, WALL_HEIGHT, 40.0]),
    // The plaza's fountain.
    ("fountain", Cover, [-2.0, 0.0, -2.0], [2.0, 0.8, 2.0]),
    // Houses around the plaza.
    ("house_northwest", Wall, [-30.0, 0.0, -30.0], [-18.0, 6.0, -20.0]),
    ("house_northeast", Wall, [18.0, 0.0, -32.0], [32.0, 7.0, -20.0]),
    ("house_southwest", Wall, [-32.0, 0.0, 18.0], [-20.0, 5.0, 30.0]),
    ("house_southeast", Wall, [20.0, 0.0, 20.0], [30.0, 6.0, 32.0]),
    // Market stalls (where vendors and the market will stand).
    ("stall_1", Cover, [-14.0, 0.0, 14.0], [-10.0, 1.2, 16.0]),
    ("stall_2", Cover, [-6.0, 0.0, 14.0], [-2.0, 1.2, 16.0]),
    ("stall_3", Cover, [2.0, 0.0, 14.0], [6.0, 1.2, 16.0]),
    ("stall_4", Cover, [10.0, 0.0, 14.0], [14.0, 1.2, 16.0]),
];

#[rustfmt::skip]
const DUNGEON_BOXES: &[MetreBox] = &[
    ("dungeon_floor", Floor, [480.0, -2.0, -12.0], [520.0, 0.0, 12.0]),
    ("dungeon_wall_north", Wall, [479.0, 0.0, -13.0], [521.0, WALL_HEIGHT, -12.0]),
    ("dungeon_wall_south", Wall, [479.0, 0.0, 12.0], [521.0, WALL_HEIGHT, 13.0]),
    ("dungeon_wall_west", Wall, [479.0, 0.0, -12.0], [480.0, WALL_HEIGHT, 12.0]),
    ("dungeon_wall_east", Wall, [520.0, 0.0, -12.0], [521.0, WALL_HEIGHT, 12.0]),
    ("dungeon_pillar_1", Pillar, [492.0, 0.0, -6.0], [494.0, WALL_HEIGHT, -4.0]),
    ("dungeon_pillar_2", Pillar, [506.0, 0.0, 4.0], [508.0, WALL_HEIGHT, 6.0]),
];

/// Where characters start in the city: a ring around the fountain, looking at it.
const CITY_SPAWN_RADIUS: f32 = 8.0;
const CITY_SPAWN_COUNT: usize = 8;
/// Arrival points of the placeholder dungeon `(x, z)`, looking east.
const DUNGEON_ARRIVALS: &[(f32, f32)] = &[(484.0, -6.0), (484.0, 0.0), (484.0, 6.0)];

pub fn crawler_city_map() -> ArenaMap {
    let pieces = CITY_BOXES
        .iter()
        .chain(DUNGEON_BOXES)
        .map(|&(name, kind, min, max)| MapPiece {
            name: name.to_string(),
            kind,
            min: min.into(),
            max: max.into(),
        })
        .collect();
    let mut spawn_points: Vec<SpawnPoint> = (0..CITY_SPAWN_COUNT)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / CITY_SPAWN_COUNT as f32;
            let (x, z) = (
                CITY_SPAWN_RADIUS * ops::sin(angle),
                CITY_SPAWN_RADIUS * ops::cos(angle),
            );
            SpawnPoint {
                feet: Vec3::new(x, 0.0, z),
                // Yaw 0 looks along -Z, so this looks from (x, z) at the fountain.
                yaw: ops::atan2(x, z),
            }
        })
        .collect();
    spawn_points.extend(DUNGEON_ARRIVALS.iter().map(|&(x, z)| SpawnPoint {
        feet: Vec3::new(x, 0.0, z),
        // Looking along +X (east, into the room).
        yaw: -std::f32::consts::FRAC_PI_2,
    }));
    ArenaMap {
        half_extent: DUNGEON_X + DUNGEON_HALF_X + 2.0,
        pieces,
        spawn_points,
        pickup_spots: Vec::new(),
        areas: vec![
            Area {
                id: AreaId("city".to_string()),
                name: "City".to_string(),
                rules: AreaRules {
                    safe: true,
                    pvp: false,
                    tier: 0,
                    max_entry_level: None,
                },
                min: Vec2::splat(-CITY_HALF),
                max: Vec2::splat(CITY_HALF),
                home: true,
            },
            Area {
                id: AreaId("dungeon_placeholder".to_string()),
                name: "Placeholder dungeon".to_string(),
                // Placeholder rules until the dungeon phase decides them.
                rules: AreaRules {
                    safe: false,
                    pvp: true,
                    tier: 1,
                    max_entry_level: Some(9),
                },
                min: Vec2::new(DUNGEON_X - DUNGEON_HALF_X, -DUNGEON_HALF_Z),
                max: Vec2::new(DUNGEON_X + DUNGEON_HALF_X, DUNGEON_HALF_Z),
                home: false,
            },
        ],
    }
}
