//! `match.*` and `assert.match_*`. None is allowed for clients: only the server and scripts run them.

use crate::config::MatchConfig;
use crate::lifecycle::{finished, new_match, running};
use crate::rules::scoreboard_for;
use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandError, CommandOutcome, CommandResult, RetiredEntity};
use retreat_gameplay_api::{
    MatchEnded, MatchPhase, MatchRestarted, MatchState, PlayerId, Scoreboard,
};

/// `match.start`: creates the match, or restarts a finished one.
pub(crate) fn start_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    match find_match(world) {
        None => {
            let present = present_players(world);
            let bundle = new_match(
                &present,
                world.resource::<MatchConfig>(),
                world.resource::<Time<Fixed>>(),
            );
            world.spawn(bundle);
        }
        Some((_, MatchState { phase }, _)) => {
            if matches!(phase, MatchPhase::Running { .. }) {
                return Err(args.failure("a match is already running; use match.restart"));
            }
            restart(world);
        }
    }
    Ok(CommandOutcome::Continue)
}

/// `match.end`: finishes the running match now.
pub(crate) fn end_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (entity, state, board) = require_match(world, args)?;
    if !matches!(state.phase, MatchPhase::Running { .. }) {
        return Err(args.failure("the match is already finished"));
    }
    let next = finished(
        &board,
        world.resource::<MatchConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    world.entity_mut(entity).insert(next);
    world.write_message(MatchEnded {
        winner: board.winner(),
    });
    Ok(CommandOutcome::Continue)
}

/// `match.restart`: a new match now, with cleared scores.
pub(crate) fn restart_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    require_match(world, args)?;
    restart(world);
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_running_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (_, state, _) = require_match(world, args)?;
    match state.phase {
        MatchPhase::Running { .. } => Ok(CommandOutcome::Continue),
        other => Err(args.failure(format!("the match is not running: {other:?}"))),
    }
}

pub(crate) fn assert_finished_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (_, state, _) = require_match(world, args)?;
    match state.phase {
        MatchPhase::Finished { .. } => Ok(CommandOutcome::Continue),
        other => Err(args.failure(format!("the match is not finished: {other:?}"))),
    }
}

pub(crate) fn assert_score_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: i32 = args.parse(1, "points")?;
    let (_, _, board) = require_match(world, args)?;
    let Some(line) = board.entry(player) else {
        return Err(args.failure(format!("player {} has no score line", player.0)));
    };
    if line.points != expected {
        return Err(args.failure(format!(
            "player {} has {} points, expected {expected}",
            player.0, line.points
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_winner_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let text: String = args.parse(0, "player|draw")?;
    let expected = if text == "draw" {
        None
    } else {
        Some(PlayerId(args.parse(0, "player|draw")?))
    };
    let (_, state, _) = require_match(world, args)?;
    match state.phase {
        MatchPhase::Finished { winner, .. } if winner == expected => Ok(CommandOutcome::Continue),
        other => Err(args.failure(format!(
            "expected the winner `{text}`, the match is {other:?}"
        ))),
    }
}

fn restart(world: &mut World) {
    let Some((entity, ..)) = find_match(world) else {
        return;
    };
    let present = present_players(world);
    let state = running(
        world.resource::<MatchConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    world
        .entity_mut(entity)
        .insert((state, scoreboard_for(&present)));
    world.write_message(MatchRestarted);
}

fn present_players(world: &mut World) -> Vec<PlayerId> {
    let mut players = world.query_filtered::<&PlayerId, Without<RetiredEntity>>();
    players.iter(world).copied().collect()
}

fn find_match(world: &mut World) -> Option<(Entity, MatchState, Scoreboard)> {
    let mut matches =
        world.query_filtered::<(Entity, &MatchState, &Scoreboard), Without<RetiredEntity>>();
    matches
        .iter(world)
        .next()
        .map(|(entity, state, board)| (entity, *state, board.clone()))
}

fn require_match(
    world: &mut World,
    args: &CommandArgs,
) -> Result<(Entity, MatchState, Scoreboard), CommandError> {
    find_match(world).ok_or_else(|| {
        args.failure("there is no match yet; it starts with the first player or with match.start")
    })
}
