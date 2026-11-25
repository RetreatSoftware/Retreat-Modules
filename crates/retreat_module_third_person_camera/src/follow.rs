//! Placing every following camera.

use crate::config::ShoulderConfig;
use crate::placement::shoulder_points;
use bevy::prelude::*;
use retreat_engine_api::{
    CameraProjection, GameCamera, NetLocalClient, NetReplaying, PhysicsShape, RetiredEntity,
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

pub(crate) fn place_cameras(world: &mut World) {
    if world.resource::<NetReplaying>().0 {
        return;
    }
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
        .query_filtered::<(Entity, &PlayerId, &Transform, Option<&LookAngles>), Without<RetiredEntity>>()
        .iter(world)
        .map(|(entity, player, transform, look)| {
            (
                entity,
                *player,
                transform.translation,
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
        let position = unblocked(world, shoulder, shoulder + back, &characters, &config);
        if let Some(mut game_camera) = world.get_mut::<GameCamera>(camera) {
            game_camera.set_if_neq(GameCamera {
                position,
                target: position + view_direction(look),
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
