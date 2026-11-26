//! `shoulder.*`.

use crate::config::ShoulderConfig;
use crate::follow::ShoulderCamera;
use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandError, CommandOutcome, CommandResult, GameCamera};
use retreat_gameplay_api::{CharacterSize, PlayerId};

pub(crate) fn follow_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let has_player = !args.rest(1).is_empty();
    args.expect_count(if has_player { 2 } else { 1 })?;
    let camera = find_camera(world, args)?;
    let player = if has_player {
        Some(PlayerId(args.parse(1, "player")?))
    } else {
        None
    };
    world
        .entity_mut(camera)
        .insert(ShoulderCamera { player, side: 1.0 });
    Ok(CommandOutcome::Continue)
}

pub(crate) fn stop_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let camera = find_camera(world, args)?;
    world.entity_mut(camera).remove::<ShoulderCamera>();
    Ok(CommandOutcome::Continue)
}

pub(crate) fn swap_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    for mut camera in world.query::<&mut ShoulderCamera>().iter_mut(world) {
        camera.side = -camera.side;
    }
    Ok(CommandOutcome::Continue)
}

/// The distance from the followed player's pivot (head) to the camera.
pub(crate) fn assert_distance_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let camera = find_camera(world, args)?;
    let least: f32 = args.parse(1, "least metres")?;
    let most: f32 = args.parse(2, "most metres")?;
    let position = world
        .get::<GameCamera>(camera)
        .map(|camera| camera.position)
        .unwrap_or_default();
    let follow = world.get::<ShoulderCamera>(camera).copied();
    let wanted = follow
        .and_then(|follow| follow.player)
        .or_else(|| retreat_gameplay_api::local_player_id(*world.resource(), *world.resource()));
    let mut players = world.query::<(&PlayerId, &Transform)>();
    let Some(center) = players
        .iter(world)
        .find(|(player, _)| Some(**player) == wanted)
        .map(|(_, transform)| transform.translation)
    else {
        return Err(args.failure("the camera follows no player"));
    };
    let height = world.resource::<ShoulderConfig>().pivot_height;
    let feet = world.resource::<CharacterSize>().feet(center);
    let distance = position.distance(feet + Vec3::Y * height);
    if !(least..=most).contains(&distance) {
        return Err(args.failure(format!(
            "the camera is {distance} m from the head, expected {least} to {most}"
        )));
    }
    Ok(CommandOutcome::Continue)
}

fn find_camera(world: &mut World, args: &CommandArgs) -> Result<Entity, CommandError> {
    let name: String = args.parse(0, "camera")?;
    let mut cameras = world.query_filtered::<(Entity, &Name), With<GameCamera>>();
    cameras
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .ok_or_else(|| args.failure(format!("there is no camera named `{name}`")))
}
