//! `assert.visuals_*`: what a script can check without looking at the picture.

use crate::config::CharacterVisualsConfig;
use crate::dress::VisualMotion;
use bevy::math::ops;
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, EntityColor, HideVisuals,
    RetiredEntity,
};
use retreat_gameplay_api::PlayerId;

pub(crate) fn assert_state_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: String = args.parse(1, "state")?;
    let (motion, _) = looks_of(world, args, player)?;
    if motion.doing.name() != expected {
        return Err(args.failure(format!(
            "player {} is shown as `{}`, expected `{expected}`",
            player.0,
            motion.doing.name()
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_facing_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: f32 = args.parse(1, "yaw")?;
    let tolerance: f32 = args.parse(2, "tolerance")?;
    let (motion, _) = looks_of(world, args, player)?;
    // The shorter way round the circle.
    let (sin, cos) = ops::sin_cos(motion.facing - expected);
    let difference = ops::atan2(sin, cos).abs();
    if difference.is_nan() || difference > tolerance {
        return Err(args.failure(format!(
            "player {} faces {}, expected {expected} within {tolerance}",
            player.0, motion.facing
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_color_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let number: usize = args.parse(1, "color number")?;
    let (_, color) = looks_of(world, args, player)?;
    let colors = &world.resource::<CharacterVisualsConfig>().colors;
    if colors.get(number) != Some(&color) {
        return Err(args.failure(format!(
            "player {} does not have color {number} of the {} configured colors",
            player.0,
            colors.len()
        )));
    }
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_hidden_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: bool = args.parse(1, "hidden")?;
    let mut players =
        world.query_filtered::<(&PlayerId, Has<HideVisuals>), Without<RetiredEntity>>();
    let Some((_, hidden)) = players.iter(world).find(|(found, _)| **found == player) else {
        return Err(args.failure(format!("there is no player {}", player.0)));
    };
    if hidden != expected {
        return Err(args.failure(format!(
            "player {} hidden is {hidden}, expected {expected}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}

fn looks_of(
    world: &mut World,
    args: &CommandArgs,
    player: PlayerId,
) -> Result<(VisualMotion, Color), CommandError> {
    let mut players =
        world.query_filtered::<(&PlayerId, &VisualMotion, &EntityColor), Without<RetiredEntity>>();
    players
        .iter(world)
        .find(|(found, ..)| **found == player)
        .map(|(_, motion, color)| (*motion, color.0))
        .ok_or_else(|| {
            args.failure(format!(
                "player {} has no looks yet (they are set in the frame after it appears)",
                player.0
            ))
        })
}
