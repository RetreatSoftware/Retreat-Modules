//! `inventory.*` and `assert.inventory`. Clients may send `inventory.use`.

use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandOutcome, CommandResult, RetiredEntity};
use retreat_gameplay_api::{
    GrantRequested, HealRequested, Health, Inventory, ItemCatalog, ItemId, ItemKind, MatchState,
    PlayerId, actor, character_of, refused,
};

/// `inventory.grant <player> <item> <count>`: for checks and servers.
pub(crate) fn grant_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let player = PlayerId(args.parse(0, "player")?);
    let item = ItemId(args.parse(1, "item")?);
    let count: u32 = args.parse(2, "count")?;
    if world.resource::<ItemCatalog>().get(&item).is_none() {
        return Err(args.failure(format!("there is no item `{item}`")));
    }
    if character_of(world, player).is_none() {
        return Err(args.failure(format!("there is no player {}", player.0)));
    }
    world.write_message(GrantRequested {
        player,
        item,
        count,
    });
    Ok(CommandOutcome::Continue)
}

pub(crate) fn use_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let item = ItemId(args.parse(0, "item")?);
    let Some(kind) = world
        .resource::<ItemCatalog>()
        .get(&item)
        .map(|item| item.kind)
    else {
        return Err(args.failure(format!("there is no item `{item}`")));
    };
    let ItemKind::Heal(amount) = kind else {
        return Err(args.failure(format!("`{item}` is not an item to use")));
    };
    let player = actor(world, args)?;
    let Some(character) = character_of(world, player) else {
        return refused(world, args, "the player has no character");
    };
    let mut matches = world.query_filtered::<&MatchState, Without<RetiredEntity>>();
    if !matches.iter(world).all(MatchState::allows_combat) {
        return refused(world, args, "the match is over");
    }
    let wounded = world
        .get::<Health>(character)
        .is_some_and(|health| health.is_alive() && health.current() < health.max());
    if !wounded {
        return refused(world, args, "the player has nothing to heal");
    }
    let Some(mut inventory) = world.get_mut::<Inventory>(character) else {
        return refused(world, args, "the player has no inventory yet");
    };
    if inventory.count(&item) == 0 {
        return refused(world, args, "the player does not carry this item");
    }
    inventory.take(&item, 1);
    world.write_message(HealRequested { player, amount });
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let player = PlayerId(args.parse(0, "player")?);
    let item = ItemId(args.parse(1, "item")?);
    let expected: u32 = args.parse(2, "count")?;
    let count = character_of(world, player)
        .and_then(|entity| world.get::<Inventory>(entity))
        .map(|inventory| inventory.count(&item))
        .ok_or_else(|| args.failure(format!("player {} has no inventory", player.0)))?;
    if count != expected {
        return Err(args.failure(format!(
            "player {} carries {count} of `{item}`, expected {expected}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}
