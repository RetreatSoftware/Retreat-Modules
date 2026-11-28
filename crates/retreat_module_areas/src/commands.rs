//! `areas.*` commands.

use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandOutcome, CommandResult, PhysicsTeleport};
use retreat_gameplay_api::{
    CharacterSize, InArea, LookAngles, MapAreas, PlayerId, SpawnPoints, character_of,
    choose_spawn_point,
};

/// `areas.move <player> <area>`: puts the player's character at a spawn point of the area.
pub(crate) fn move_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let target: String = args.parse(1, "area")?;
    let areas = world.resource::<MapAreas>().clone();
    let Some(area) = areas.get(&target).cloned() else {
        let known: Vec<&str> = areas.0.iter().map(|area| area.id.0.as_str()).collect();
        return Err(args.failure(format!(
            "no area `{target}`; areas: {}",
            if known.is_empty() {
                "none (the map has no areas)".to_string()
            } else {
                known.join(", ")
            }
        )));
    };
    let Some(character) = character_of(world, player) else {
        return Err(args.failure(format!("player {} has no character", player.0)));
    };
    let points = MapAreas::spawn_points_in(&world.resource::<SpawnPoints>().0, &area);
    let size = *world.resource::<CharacterSize>();
    let mut others = world.query::<(Entity, &Transform, &PlayerId)>();
    let occupied: Vec<Vec3> = others
        .iter(world)
        .filter(|(entity, ..)| *entity != character)
        .map(|(_, transform, _)| size.feet(transform.translation))
        .collect();
    let Some(index) = choose_spawn_point(&points, &occupied) else {
        return Err(args.failure(format!(
            "area `{target}` has no spawn point to arrive at; add one to the map"
        )));
    };
    let arrival = points[index];
    let mut entity = world.entity_mut(character);
    entity.insert((
        PhysicsTeleport(size.center(arrival.feet)),
        InArea(area.id.clone()),
    ));
    if let Some(mut look) = entity.get_mut::<LookAngles>() {
        look.yaw = arrival.yaw;
    }
    Ok(CommandOutcome::Continue)
}

/// `areas.list`: logs every area and its rules.
pub(crate) fn list_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let areas = world.resource::<MapAreas>();
    if areas.0.is_empty() {
        info!("areas: none (the map has no areas)");
    }
    for area in &areas.0 {
        info!(
            "area `{}` \"{}\": safe {}, pvp {}, tier {}, max entry level {}, x {}..{}, z {}..{}{}",
            area.id.0,
            area.name,
            area.rules.safe,
            area.rules.pvp,
            area.rules.tier,
            area.rules
                .max_entry_level
                .map_or("none".to_string(), |level| level.to_string()),
            area.min.x,
            area.max.x,
            area.min.y,
            area.max.y,
            if area.home { ", home" } else { "" }
        );
    }
    Ok(CommandOutcome::Continue)
}

/// `assert.area <player> <area>`.
pub(crate) fn assert_area_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: String = args.parse(1, "area")?;
    let Some(character) = character_of(world, player) else {
        return Err(args.failure(format!("player {} has no character", player.0)));
    };
    match world.get::<InArea>(character) {
        Some(area) if area.0.0 == expected => Ok(CommandOutcome::Continue),
        Some(area) => Err(args.failure(format!(
            "player {} is in area `{}`, expected `{expected}`",
            player.0, area.0.0
        ))),
        None => Err(args.failure(format!("player {} is in no area yet", player.0))),
    }
}
