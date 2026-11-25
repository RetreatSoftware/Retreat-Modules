//! What a map is, and the pure rules a map must satisfy.

use bevy::prelude::*;
use retreat_gameplay_api::{CHARACTER_HEIGHT, CHARACTER_RADIUS, MAX_PLAYERS, SpawnPoint};

/// Surfaces closer than this are touching, not overlapping (metres).
const TOUCH_TOLERANCE: f32 = 1.0e-3;

/// What a piece is for; decides its graybox color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    Floor,
    Wall,
    Platform,
    Stair,
    Cover,
    Pillar,
}

/// One axis-aligned box of a map, in metres, Y up.
#[derive(Debug, Clone, PartialEq)]
pub struct MapPiece {
    /// Unique in the map; lowercase, digits and `_`.
    pub name: String,
    pub kind: PieceKind,
    pub min: Vec3,
    pub max: Vec3,
}

impl MapPiece {
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

/// A whole map.
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaMap {
    /// The playable floor spans `-half_extent..=half_extent` on x and z.
    pub half_extent: f32,
    pub pieces: Vec<MapPiece>,
    /// At least [`MAX_PLAYERS`], so everyone can start somewhere different.
    pub spawn_points: Vec<SpawnPoint>,
}

/// Why a character cannot appear at `spawn`, or `None` when it can: its bounding box (a little
/// larger than its capsule) must be inside the arena, overlap no piece and rest on one.
pub fn spawn_point_problem(map: &ArenaMap, spawn: &SpawnPoint) -> Option<String> {
    if !(spawn.feet.is_finite() && spawn.yaw.is_finite()) {
        return Some("position and yaw must be finite numbers".to_string());
    }
    let feet = spawn.feet;
    let min = feet - Vec3::new(CHARACTER_RADIUS, 0.0, CHARACTER_RADIUS);
    let max = feet + Vec3::new(CHARACTER_RADIUS, CHARACTER_HEIGHT, CHARACTER_RADIUS);
    let limit = map.half_extent + TOUCH_TOLERANCE;
    if min.x < -limit || min.z < -limit || max.x > limit || max.z > limit {
        return Some("is outside the arena".to_string());
    }
    if let Some(piece) = map.pieces.iter().find(|piece| {
        let overlap = max.min(piece.max) - min.max(piece.min);
        overlap.min_element() > TOUCH_TOLERANCE
    }) {
        return Some(format!("is inside `{}`", piece.name));
    }
    let supported = map.pieces.iter().any(|piece| {
        (piece.max.y - feet.y).abs() <= TOUCH_TOLERANCE
            && (piece.min.x..=piece.max.x).contains(&feet.x)
            && (piece.min.z..=piece.max.z).contains(&feet.z)
    });
    (!supported).then(|| "does not stand on any piece".to_string())
}

/// Every problem of a map, each saying what to fix. Empty means the map can be loaded.
pub fn validate_map(map: &ArenaMap) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, piece) in map.pieces.iter().enumerate() {
        let name = &piece.name;
        if map.pieces[..index].iter().any(|other| other.name == *name) {
            problems.push(format!("piece name `{name}` is used twice"));
        }
        if !(piece.min.is_finite() && piece.max.is_finite() && piece.size().min_element() > 0.0) {
            problems.push(format!(
                "piece `{name}` needs finite corners with min below max on every axis"
            ));
        }
    }
    if map.spawn_points.len() < MAX_PLAYERS {
        problems.push(format!(
            "{} spawn points, at least {MAX_PLAYERS} are needed",
            map.spawn_points.len()
        ));
    }
    for (index, spawn) in map.spawn_points.iter().enumerate() {
        if let Some(problem) = spawn_point_problem(map, spawn) {
            problems.push(format!("spawn point {index} at {} {problem}", spawn.feet));
        }
        let too_close = map.spawn_points[..index]
            .iter()
            .any(|other| other.feet.distance(spawn.feet) < 2.0 * CHARACTER_RADIUS);
        if too_close {
            problems.push(format!("spawn point {index} overlaps an earlier one"));
        }
    }
    problems
}
