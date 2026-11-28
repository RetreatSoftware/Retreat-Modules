//! Grid inventory (crawler plan, phase 2): every playing character's bag, a grid of
//! [`GridInventoryConfig::columns`] x [`GridInventoryConfig::rows`] cells, the same size for everyone
//! (decided with the user: no backpacks; the size is placeholder config). Items take `W x H` cells
//! and may be turned 90 degrees; stackable items (gold) stack up to their base's `max_stack`.
//!
//! The bag is the contract container `bag/<character>` (entity `container_bag_<character>`), opened
//! with its saved items on the authoritative machine as soon as the character plays and removed when
//! it stops; private to the owning player. Its items are saved in this module's record part
//! `grid_inventory` of `character/<id>`, in the contract's container format, on every change (the
//! contract's atomic item transaction).
//!
//! Commands (player requests, routed to the server): `bag.put <uid> <x> <y> <0|1>` (the bag's drop:
//! move an item there from any of the character's containers, or onto a stack of the same kind),
//! `bag.split <uid> <amount>` (a new stack of `amount` at the first free place). Refusals are the
//! player's item notice. Assertions (this machine's view): `assert.bag_items <count>`,
//! `assert.bag_quantity <base> <total>`.
//!
//! Gold found is a carried item (a stack in the bag, lost on death in phase 4) until the wallet
//! module deposits it. Dropping items on the ground comes with world loot (phase 3).

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppNetExt, CommandArgs, CommandOutcome, CommandResult, NetLocalClient,
};
use retreat_gameplay_api::{
    CharacterId, ContainerKind, ContainerLayout, GameplaySystems, ItemContainer, ItemTransaction,
    ItemUid, PlacedItem, PlayingCharacter, SoloPlayer, actor, add_gameplay_contract,
    allocate_item_uids, apply_item_transaction, character_record_owner, container_id,
    container_items, first_free_spot, keep_character_containers, local_player_id, placed_item,
    playing_body, put_request, refused, route_request, show_item_notice, sync_item_privacy,
};

/// This module's record part (and id).
pub const BAG_RECORD_PART: &str = "grid_inventory";

/// The bag size (placeholder numbers; insert before adding the plugin to change).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GridInventoryConfig {
    pub columns: u32,
    pub rows: u32,
}

impl Default for GridInventoryConfig {
    fn default() -> Self {
        Self {
            columns: 8,
            rows: 5,
        }
    }
}

/// Adds bags and the `bag.*` commands.
pub struct GridInventoryModulePlugin;

impl Plugin for GridInventoryModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<GridInventoryConfig>();
        let config = app.world().resource::<GridInventoryConfig>().clone();
        assert!(
            config.columns > 0 && config.rows > 0,
            "invalid GridInventoryConfig: the bag needs at least one column and one row"
        );
        app.register_command("bag.put", "bag.put <uid> <x> <y> <0|1>", put_command)
            .allow_client_command("bag.put")
            .register_command("bag.split", "bag.split <uid> <amount>", split_command)
            .allow_client_command("bag.split")
            .register_command(
                "assert.bag_items",
                "assert.bag_items <count>",
                assert_items_command,
            )
            .register_command(
                "assert.bag_quantity",
                "assert.bag_quantity <base> <total>",
                assert_quantity_command,
            )
            .add_systems(FixedUpdate, keep_bags.in_set(GameplaySystems::Items));
    }
}

/// The bag of `character`.
pub fn bag_container(config: &GridInventoryConfig, character: &CharacterId) -> ItemContainer {
    ItemContainer {
        id: container_id(ContainerKind::Bag, character),
        kind: ContainerKind::Bag,
        title: "Bag".to_string(),
        layout: ContainerLayout::Grid {
            columns: config.columns,
            rows: config.rows,
            visible_rows: 0,
        },
        character: character.clone(),
        only_in_safe_area: false,
        drop_command: "bag.put".to_string(),
        record_owner: character_record_owner(character),
        record_part: BAG_RECORD_PART.to_string(),
    }
}

/// Every playing character has its bag; bags of characters that left go; privacy follows takeovers.
fn keep_bags(world: &mut World) {
    let config = world.resource::<GridInventoryConfig>().clone();
    keep_character_containers(world, ContainerKind::Bag, |playing: &PlayingCharacter| {
        bag_container(&config, &playing.id)
    });
    sync_item_privacy(world);
}

/// `bag.put <uid> <x> <y> <0|1>`.
fn put_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    put_request(world, args, "bag.put", ContainerKind::Bag)
}

/// `bag.split <uid> <amount>`: takes `amount` off a stack in the bag into a new stack.
fn split_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let uid: u64 = args.parse(0, "item uid")?;
    let amount: u32 = args.parse(1, "amount")?;
    if !route_request(world, format!("bag.split {uid} {amount}")) {
        return Ok(CommandOutcome::Continue);
    }
    let player = actor(world, args)?;
    let Some((body, playing)) = playing_body(world, player) else {
        return refused(world, args, "you are not playing a character");
    };
    let result = split(world, &playing.id, ItemUid(uid), amount)
        .and_then(|transaction| apply_item_transaction(world, player, transaction));
    match result {
        Ok(()) => Ok(CommandOutcome::Continue),
        Err(why) => {
            show_item_notice(world, body, why.clone());
            refused(world, args, &why)
        }
    }
}

/// The transaction of a split, or why not.
fn split(
    world: &mut World,
    character: &CharacterId,
    uid: ItemUid,
    amount: u32,
) -> Result<ItemTransaction, String> {
    let bag = container_id(ContainerKind::Bag, character);
    let stack = placed_item(world, uid)
        .filter(|placed| placed.place.container == bag)
        .ok_or("that item is not in your bag")?;
    if amount == 0 || amount >= stack.item.quantity {
        return Err(format!(
            "split 1 to {} off a stack of {}",
            stack.item.quantity.saturating_sub(1),
            stack.item.quantity
        ));
    }
    let config = world.resource::<GridInventoryConfig>().clone();
    let items = container_items(world, &bag);
    let (at, rotated) = first_free_spot(config.columns, config.rows, &items, stack.item.size)
        .ok_or("the bag has no room for another stack")?;
    let new_uid = allocate_item_uids(world, 1)?[0];
    let mut rest = stack.clone();
    rest.item.quantity -= amount;
    let mut split_off = stack;
    split_off.item.uid = new_uid;
    split_off.item.quantity = amount;
    split_off.place.x = at.x;
    split_off.place.y = at.y;
    split_off.place.rotated = rotated;
    Ok(ItemTransaction {
        put: vec![rest, split_off],
        ..Default::default()
    })
}

/// The items of this machine's player's bag (as this machine sees them).
fn own_bag_items(
    world: &mut World,
    args: &CommandArgs,
) -> Result<Vec<PlacedItem>, retreat_engine_api::CommandError> {
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
    .map_or_else(|| actor(world, args), Ok)?;
    let (_, playing) = playing_body(world, player)
        .ok_or_else(|| args.failure("this machine's player is not playing a character"))?;
    Ok(container_items(
        world,
        &container_id(ContainerKind::Bag, &playing.id),
    ))
}

/// `assert.bag_items <count>`.
fn assert_items_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "count")?;
    let items = own_bag_items(world, args)?;
    if items.len() == expected {
        Ok(CommandOutcome::Continue)
    } else {
        let bases: Vec<&str> = items.iter().map(|p| p.item.base.as_str()).collect();
        Err(args.failure(format!(
            "the bag holds {} items {bases:?}, expected {expected}",
            items.len()
        )))
    }
}

/// `assert.bag_quantity <base> <total>`: the stacks of `base` in the bag add up to `total`.
fn assert_quantity_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let base: String = args.parse(0, "base")?;
    let expected: u32 = args.parse(1, "total")?;
    let total: u32 = own_bag_items(world, args)?
        .iter()
        .filter(|placed| placed.item.base == base)
        .map(|placed| placed.item.quantity)
        .sum();
    if total == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!("the bag holds {total} {base}, expected {expected}")))
    }
}
