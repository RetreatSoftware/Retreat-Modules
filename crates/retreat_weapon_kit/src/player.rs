//! Who a weapon command acts for, and who may shoot.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, NetCommandSender, NetLocalClient,
    RetiredEntity,
};
use retreat_gameplay_api::{Health, MatchState, PlayerId, SoloPlayer, local_player_id};

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

/// Dead players are not there for a shot (the present decides, not the rewound past).
pub fn dead_players(world: &mut World) -> Vec<Entity> {
    let mut players =
        world.query_filtered::<(Entity, &Health), (With<PlayerId>, Without<RetiredEntity>)>();
    players
        .iter(world)
        .filter(|(_, health)| !health.is_alive())
        .map(|(entity, _)| entity)
        .collect()
}

/// Alive, and no finished match forbids fighting.
pub fn may_fight(world: &mut World, character: Entity) -> bool {
    let alive = world.get::<Health>(character).is_none_or(Health::is_alive);
    let mut matches = world.query_filtered::<&MatchState, Without<RetiredEntity>>();
    alive && matches.iter(world).all(MatchState::allows_combat)
}
