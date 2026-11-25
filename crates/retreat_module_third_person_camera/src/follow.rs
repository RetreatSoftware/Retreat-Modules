//! Placing every following camera.

use crate::config::ShoulderConfig;
use crate::placement::{closeness, shoulder_points, tilted};
use bevy::prelude::*;
use retreat_engine_api::{
    CameraProjection, DrawnPose, GameCamera, NetLocalClient, PhysicsShape, RetiredEntity,
    ShapeCast, cast_shape,
};
use retreat_gameplay_api::{LookAngles, PlayerId, SoloPlayer, local_player_id, view_direction};

/// On a camera that follows a player.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCamera {
    /// `None` follows this machine's player.
    pub player: Option<PlayerId>,
    /// 1 right shoulder, -1 left.
    pub side: f32,
}

type Undrawn = (With<PlayerId>, Without<DrawnPose>);

/// Asks the engine where each player is drawn: the camera follows the picture, not the tick.
pub(crate) fn track_drawn_poses(
    players: Query<(Entity, &Transform), Undrawn>,
    mut commands: Commands,
) {
    for (entity, transform) in &players {
        commands.entity(entity).insert(DrawnPose {
            translation: transform.translation,
            rotation: transform.rotation,
        });
    }
}

pub(crate) fn place_cameras(world: &mut World) {
    let local = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    );
    let config = world.resource::<ShoulderConfig>().clone();
    let cameras: Vec<(Entity, ShoulderCamera)> = world
        .query::<(Entity, &ShoulderCamera)>()
        .iter(world)
        .map(|(entity, camera)| (entity, *camera))
        .collect();
    if cameras.is_empty() {
        return;
    }
    let players: Vec<(Entity, PlayerId, Vec3, LookAngles)> = world
        .query_filtered::<(Entity, &PlayerId, &DrawnPose, Option<&LookAngles>), Without<RetiredEntity>>()
        .iter(world)
        .map(|(entity, player, drawn, look)| {
            (
                entity,
                *player,
                drawn.translation,
                look.copied().unwrap_or_default(),
            )
        })
        .collect();
    let characters: Vec<Entity> = players.iter().map(|(entity, ..)| *entity).collect();
    for (camera, follow) in cameras {
        let Some(wanted) = follow.player.or(local) else {
            continue;
        };
        let Some(&(_, _, center, look)) = players.iter().find(|(_, player, ..)| *player == wanted)
        else {
            continue;
        };
        let points = shoulder_points(center, look, follow.side, &config);
        let shoulder = unblocked(world, points.pivot, points.shoulder, &characters, &config);
        let back = points.camera - points.shoulder;
        let behind = unblocked(world, shoulder, shoulder + back, &characters, &config);
        let close = closeness(behind.distance(shoulder), &config);
        let mut position = behind;
        let mut view = look;
        if close > 0.0 {
            // Pushed against a wall: over the head instead of the shoulder, looking down.
            let beside = (shoulder - points.pivot) * (1.0 - close) + (behind - shoulder);
            let level = unblocked(
                world,
                points.pivot,
                points.pivot + beside,
                &characters,
                &config,
            );
            let raised = level + Vec3::Y * (config.close_lift * close);
            position = unblocked(world, level, raised, &characters, &config);
            view = tilted(look, close, &config);
        }
        if let Some(mut game_camera) = world.get_mut::<GameCamera>(camera) {
            game_camera.set_if_neq(GameCamera {
                position,
                target: position + view_direction(view),
                up: Vec3::Y,
                projection: CameraProjection::Perspective {
                    fov_degrees: config.fov_degrees,
                },
            });
        }
    }
}

/// How far from `from` towards `to` a ball of the wall margin gets.
fn unblocked(
    world: &mut World,
    from: Vec3,
    to: Vec3,
    ignored: &[Entity],
    config: &ShoulderConfig,
) -> Vec3 {
    let path = to - from;
    let length = path.length();
    if length <= f32::EPSILON {
        return to;
    }
    let hit = cast_shape(
        world,
        &ShapeCast {
            shape: PhysicsShape::Ball {
                radius: config.wall_margin,
            },
            origin: from,
            rotation: Quat::IDENTITY,
            direction: path,
            max_distance: length,
            exclude: ignored.to_vec(),
        },
    );
    hit.map_or(to, |hit| from + path / length * hit.distance)
}
