//! `spawn.*`. None is allowed for clients: joining the server is what spawns a client's character.

use crate::spawn_character;
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandOutcome, CommandResult, RemovePhysicsBody, RetiredEntity,
};
use retreat_gameplay_api::{PlayerId, SoloPlayer};

pub(crate) fn join_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let player = PlayerId(args.parse(0, "player")?);
    spawn_character(world, player).map_err(|problem| args.failure(problem))?;
    Ok(CommandOutcome::Continue)
}

pub(crate) fn solo_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let player = PlayerId(args.parse(0, "player")?);
    if let Some(current) = world.resource::<SoloPlayer>().0 {
        return Err(args.failure(format!("player {} is already the solo player", current.0)));
    }
    spawn_character(world, player).map_err(|problem| args.failure(problem))?;
    world.insert_resource(SoloPlayer(Some(player)));
    Ok(CommandOutcome::Continue)
}

pub(crate) fn leave_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let player = PlayerId(args.parse(0, "player")?);
    let mut players = world.query_filtered::<(Entity, &PlayerId), Without<RetiredEntity>>();
    let Some(entity) = players
        .iter(world)
        .find(|(_, found)| **found == player)
        .map(|(entity, _)| entity)
    else {
        return Err(args.failure(format!("there is no player {}", player.0)));
    };
    world.entity_mut(entity).insert(RemovePhysicsBody);
    if world.resource::<SoloPlayer>().0 == Some(player) {
        world.insert_resource(SoloPlayer(None));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_count_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "characters")?;
    // Characters leaving this tick are already marked for removal.
    let mut players = world.query_filtered::<(), (
        With<PlayerId>,
        Without<RetiredEntity>,
        Without<RemovePhysicsBody>,
    )>();
    let actual = players.iter(world).count();
    if actual != expected {
        return Err(args.failure(format!("there are {actual} players, expected {expected}")));
    }
    Ok(CommandOutcome::Continue)
}
