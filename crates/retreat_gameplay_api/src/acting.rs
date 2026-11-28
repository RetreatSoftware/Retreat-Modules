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

/// Where a player request command goes (the crawler's routing rule, see the modules' CLAUDE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// A network client's own request (a button, a field, a script line, a drop): send it to the
    /// server and do nothing else.
    SendToServer,
    /// A network client running its own request as a prediction: the server decides, so nothing.
    Ignore,
    /// The server, or a game without a network: decide, for the sender.
    Decide,
}

/// The route of the command being run now.
pub fn route(world: &World) -> Route {
    let is_client = world.resource::<NetInputQueue>().accepting;
    let sent_by = world.resource::<NetCommandSender>().0;
    match (is_client, sent_by) {
        (true, None) => Route::SendToServer,
        (true, Some(_)) => Route::Ignore,
        (false, _) => Route::Decide,
    }
}

/// Routes the command line `line` that is being run (see [`route`]): `true` when this machine
/// decides it, `false` when it was sent to the server or is a client's prediction (nothing to do).
pub fn route_request(world: &mut World, line: String) -> bool {
    match route(world) {
        Route::SendToServer => {
            world.resource_mut::<NetInputQueue>().push(line);
            false
        }
        Route::Ignore => false,
        Route::Decide => true,
    }
}
