//! Command helpers every module that takes player input needs: who a command acts for, how a
//! refusal is answered, and how this machine's input becomes a command line.

use crate::{PlayerId, SoloPlayer, local_player_id};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, NetCommandSender, NetInputQueue,
    NetLocalClient, RetiredEntity, run_command_line,
};

/// The player a command acts for: who sent it, otherwise this machine's player.
pub fn actor(world: &World, args: &CommandArgs) -> Result<PlayerId, CommandError> {
    if let Some(id) = world.resource::<NetCommandSender>().0 {
        return Ok(PlayerId(id));
    }
    local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
    .ok_or_else(|| {
        args.failure("this machine has no player; send it from a client or use spawn.solo")
    })
}

/// A refusal: normal for a network input (it can cross a death or a reload), an error in a script.
pub fn refused(world: &World, args: &CommandArgs, why: &str) -> CommandResult {
    if world.resource::<NetCommandSender>().0.is_some() {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(why.to_string()))
    }
}

/// The character of `player`, if there is one.
pub fn character_of(world: &mut World, player: PlayerId) -> Option<Entity> {
    let mut players = world.query_filtered::<(Entity, &PlayerId), Without<RetiredEntity>>();
    players
        .iter(world)
        .find(|(_, found)| **found == player)
        .map(|(entity, _)| entity)
}

/// Sends this machine's input line to the server, or runs it here without a network.
pub fn issue(world: &mut World, line: String) {
    if world.resource::<NetInputQueue>().accepting {
        world.resource_mut::<NetInputQueue>().push(line);
    } else if let Err(error) = run_command_line(world, &line) {
        warn!("{error}");
    }
}
