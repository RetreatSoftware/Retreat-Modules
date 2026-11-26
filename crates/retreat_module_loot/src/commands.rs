//! `assert.loot_*`.

use crate::state::{Pickup, PickupOrigin};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandOutcome, CommandResult, RemoveNetEntity, RetiredEntity,
};

/// `assert.loot_available <spot> <true|false>`.
pub(crate) fn assert_available_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let spot: String = args.parse(0, "spot")?;
    let expected: bool = args.parse(1, "there")?;
    let wanted = format!("pickup/{spot}");
    let mut pickups = world.query_filtered::<(&Name, &Pickup), Without<RetiredEntity>>();
    let found = pickups
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .map(|(_, pickup)| pickup.is_there())
        .ok_or_else(|| args.failure(format!("there is no pickup at `{spot}`")))?;
    if found != expected {
        return Err(args.failure(format!(
            "the pickup at `{spot}` is there: {found}, expected {expected}"
        )));
    }
    Ok(CommandOutcome::Continue)
}

/// `assert.loot_drops <count>`.
pub(crate) fn assert_drops_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "count")?;
    let mut pickups =
        world.query_filtered::<&Pickup, (Without<RetiredEntity>, Without<RemoveNetEntity>)>();
    let drops = pickups
        .iter(world)
        .filter(|pickup| matches!(pickup.origin, PickupOrigin::Drop { .. }))
        .count();
    if drops != expected {
        return Err(args.failure(format!("{drops} drops lie around, expected {expected}")));
    }
    Ok(CommandOutcome::Continue)
}
