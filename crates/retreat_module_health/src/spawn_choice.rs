//! Where to respawn.

use bevy::prelude::*;
use retreat_gameplay_api::SpawnPoint;

/// The index of the point farthest from the nearest position in `occupied`; the first such point on
/// a tie (so the first point when nobody is around). `None` when there are no points.
pub fn choose_spawn_point(points: &[SpawnPoint], occupied: &[Vec3]) -> Option<usize> {
    let clearance = |point: &SpawnPoint| {
        occupied
            .iter()
            .map(|position| position.distance_squared(point.feet))
            .fold(f32::INFINITY, f32::min)
    };
    let mut best: Option<(usize, f32)> = None;
    for (index, point) in points.iter().enumerate() {
        let distance = clearance(point);
        if best.is_none_or(|(_, farthest)| distance > farthest) {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index)
}
