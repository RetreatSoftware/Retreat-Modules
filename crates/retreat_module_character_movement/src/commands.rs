//! `movement.*` and `assert.movement_*`. Clients may send only `movement.input`.

use crate::config::MovementConfig;
use crate::input::LocalAim;
use crate::state::{MovementState, PlayerInput};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, NetCommandSender, NetLocalClient,
    PhysicsVelocity, RetiredEntity,
};
use retreat_gameplay_api::{CharacterSize, LookAngles, PlayerId, SoloPlayer, local_player_id};

/// `movement.input`: the input of the player who sent it (without a sender, this machine's player).
pub(crate) fn input_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(7)?;
    let input = PlayerInput {
        strafe: axis(args, 0, "strafe")?,
        advance: axis(args, 1, "advance")?,
        sprint: button(args, 4, "sprint")?,
        jump: button(args, 5, "jump")?,
        dash: button(args, 6, "dash")?,
    };
    let yaw = finite(args, 2, "yaw")?;
    let pitch = finite(args, 3, "pitch")?;
    let sender = world.resource::<NetCommandSender>().0;
    let player = match sender {
        Some(id) => PlayerId(id),
        None => local_player(world).ok_or_else(|| {
            args.failure("this machine has no player; send it from a client or use spawn.solo")
        })?,
    };
    let max_pitch = world.resource::<MovementConfig>().max_pitch;
    let mut players = world
        .query_filtered::<(&PlayerId, &mut MovementState, &mut LookAngles), Without<RetiredEntity>>(
        );
    let Some((_, mut state, mut look)) = players
        .iter_mut(world)
        .find(|(found, ..)| **found == player)
    else {
        // Normal for a client: its inputs can cross its character appearing or leaving.
        if sender.is_some() {
            return Ok(CommandOutcome::Continue);
        }
        return Err(args.failure(format!("player {} has no character", player.0)));
    };
    state.input = input;
    *look = LookAngles {
        yaw,
        pitch: pitch.clamp(-max_pitch, max_pitch),
    };
    Ok(CommandOutcome::Continue)
}

/// `movement.look`: turns this machine's view, as the mouse would.
pub(crate) fn look_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let yaw = finite(args, 0, "yaw")?;
    let pitch = finite(args, 1, "pitch")?;
    if local_player(world).is_none() {
        return Err(args.failure("this machine has no player"));
    }
    let max_pitch = world.resource::<MovementConfig>().max_pitch;
    world.insert_resource(LocalAim(Some(LookAngles {
        yaw,
        pitch: pitch.clamp(-max_pitch, max_pitch),
    })));
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_grounded_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: bool = args.parse(1, "grounded")?;
    let (state, ..) = character(world, args, player)?;
    if state.grounded != expected {
        return Err(args.failure(format!(
            "player {} grounded is {}, expected {expected}",
            player.0, state.grounded
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_speed_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected = finite(args, 1, "metres per second")?;
    let tolerance = finite(args, 2, "tolerance")?;
    let (_, _, velocity) = character(world, args, player)?;
    let speed = velocity.xz().length();
    if (speed - expected).abs() > tolerance {
        return Err(args.failure(format!(
            "player {} moves at {speed} m/s along the ground, expected {expected} within {tolerance}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_near_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(5)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected = Vec3::new(
        finite(args, 1, "feet x")?,
        finite(args, 2, "feet y")?,
        finite(args, 3, "feet z")?,
    );
    let tolerance = finite(args, 4, "tolerance")?;
    let (_, center, _) = character(world, args, player)?;
    let feet = world.resource::<CharacterSize>().feet(center);
    if feet.distance(expected) > tolerance {
        return Err(args.failure(format!(
            "player {} stands at {feet}, expected {expected} within {tolerance}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}

/// The view the simulation has for a player (a script waits on it to hear another client).
pub(crate) fn assert_look_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected = finite(args, 1, "yaw")?;
    let tolerance = finite(args, 2, "tolerance")?;
    let mut players = world.query_filtered::<(&PlayerId, &LookAngles), Without<RetiredEntity>>();
    let Some((_, look)) = players.iter(world).find(|(found, _)| **found == player) else {
        return Err(args.failure(format!("player {} has no view", player.0)));
    };
    // The shorter way round the circle.
    let turn = std::f32::consts::TAU;
    let difference = (look.yaw - expected).rem_euclid(turn);
    if difference.min(turn - difference) > tolerance {
        return Err(args.failure(format!(
            "player {} looks towards yaw {}, expected {expected} within {tolerance}",
            player.0, look.yaw
        )));
    }
    Ok(CommandOutcome::Continue)
}

fn local_player(world: &World) -> Option<PlayerId> {
    local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
}

/// State, position and velocity of a player's character.
fn character(
    world: &mut World,
    args: &CommandArgs,
    player: PlayerId,
) -> Result<(MovementState, Vec3, Vec3), CommandError> {
    let mut players = world.query_filtered::<(
        &PlayerId,
        &MovementState,
        &Transform,
        &PhysicsVelocity,
    ), Without<RetiredEntity>>();
    players
        .iter(world)
        .find(|(found, ..)| **found == player)
        .map(|(_, state, transform, velocity)| (*state, transform.translation, velocity.linear))
        .ok_or_else(|| args.failure(format!("player {} has no character", player.0)))
}

fn finite(args: &CommandArgs, index: usize, name: &str) -> Result<f32, CommandError> {
    let value: f32 = args.parse(index, name)?;
    if !value.is_finite() {
        return Err(args.failure(format!("<{name}> must be a finite number, got {value}")));
    }
    Ok(value)
}

fn axis(args: &CommandArgs, index: usize, name: &str) -> Result<f32, CommandError> {
    Ok(finite(args, index, name)?.clamp(-1.0, 1.0))
}

fn button(args: &CommandArgs, index: usize, name: &str) -> Result<bool, CommandError> {
    match args.parse::<u8>(index, name)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(args.failure(format!("<{name}> must be 0 or 1, got {other}"))),
    }
}
