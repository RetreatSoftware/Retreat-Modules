//! The one atomic item operation and the world helpers item modules share (crawler plan, phase 2).
//!
//! **Atomic transfers** ([`ItemTransaction`], [`apply_item_transaction`]): every change of items
//! (a move between two owners' containers, a stack merge, new items, a gold deposit with the new
//! balance) is first computed as data, then **all** touched containers' records (and any extra
//! records, such as a balance) are written in one `RecordStore` batch, and only when that write
//! succeeded is the world changed. A crash therefore leaves either the old or the new state on disk,
//! never an item in two places or in none. Containers are saved in the contract's format
//! ([`container_record`]) under their own `record_owner` / `record_part`, whichever module moves the
//! items (decided 2026-11-28: one format keeps cross-module moves atomic; the owning module still
//! loads, shows and removes its containers).
//!
//! **Unique ids** ([`allocate_item_uids`]): uids come from a counter whose reservation is saved
//! (record `items` / `item_uids`) **before** any uid of the block is handed out, so a restart never
//! hands out a uid twice; a crash only skips the rest of a block.
//!
//! These helpers take the `World` because they act for a command; they hold no state between ticks
//! except the uid counter (a server-only resource, not simulation state).

use std::collections::BTreeMap;

use bevy::prelude::*;
use retreat_engine_api::serde_json::{self, Value, json};
use retreat_engine_api::{
    NetPrivateTo, RecordBatch, RecordKey, RecordStore, RemoveNetEntity, RemovePhysicsBody,
    RetiredEntity,
};
use serde::{Deserialize, Serialize};

use crate::{
    CharacterId, CharacterLevel, ContainerId, GridItem, GridItemCatalog, InArea, ItemContainer,
    ItemPlace, ItemUid, MapAreas, MoveContext, MoveOutcome, PlacedItem, PlayerId, PlayingCharacter,
    container_entity_name, item_entity_name, plan_move,
};

/// The data version of a saved container.
pub const CONTAINER_RECORD_VERSION: u32 = 1;
/// How many uids one saved reservation covers.
pub const ITEM_UID_BLOCK: u64 = 1000;
/// Where the uid reservation is saved.
pub const ITEM_UID_RECORD_OWNER: &str = "items";
pub const ITEM_UID_RECORD_PART: &str = "item_uids";

/// A short message for the player about their items (a refusal), on the character's body (shared).
/// `serial` grows with each message, so the same text twice is shown twice.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ItemNotice {
    pub text: String,
    pub serial: u32,
}

impl retreat_engine_api::NetState for ItemNotice {
    const ID: &'static str = "gameplay.item_notice";
}

/// An entity by its unique `Name` (retired ones excluded).
pub fn named_entity(world: &mut World, name: &str) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &Name), (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
}

/// The live body of `player` that plays a saved character, and that character.
pub fn playing_body(world: &mut World, player: PlayerId) -> Option<(Entity, PlayingCharacter)> {
    world
        .query_filtered::<(Entity, &PlayerId, &PlayingCharacter), (
            Without<RetiredEntity>,
            Without<RemovePhysicsBody>,
        )>()
        .iter(world)
        .find(|(_, found, _)| **found == player)
        .map(|(entity, _, character)| (entity, character.clone()))
}

/// The live body playing `character`, and its player.
pub fn body_of_character(world: &mut World, character: &CharacterId) -> Option<(Entity, PlayerId)> {
    world
        .query_filtered::<(Entity, &PlayerId, &PlayingCharacter), (
            Without<RetiredEntity>,
            Without<RemovePhysicsBody>,
        )>()
        .iter(world)
        .find(|(_, _, playing)| playing.id == *character)
        .map(|(entity, player, _)| (entity, *player))
}

/// The level of a body (1 without [`CharacterLevel`]).
pub fn character_level(world: &World, body: Entity) -> u32 {
    world.get::<CharacterLevel>(body).map_or(1, |level| level.0)
}

/// Whether a body stands in a safe area. A map without areas is all safe.
pub fn in_safe_area(world: &World, body: Entity) -> bool {
    let areas = world.resource::<MapAreas>();
    if areas.0.is_empty() {
        return true;
    }
    world
        .get::<InArea>(body)
        .and_then(|in_area| areas.get(&in_area.0.0))
        .is_some_and(|area| area.rules.safe)
}

/// The entity of a container.
pub fn container_entity(world: &mut World, id: &ContainerId) -> Option<Entity> {
    named_entity(world, &container_entity_name(id))
}

/// A container's description.
pub fn container_of(world: &mut World, id: &ContainerId) -> Option<ItemContainer> {
    let entity = container_entity(world, id)?;
    world.get::<ItemContainer>(entity).cloned()
}

/// The items in a container, by uid.
pub fn container_items(world: &mut World, id: &ContainerId) -> Vec<PlacedItem> {
    let mut items: Vec<PlacedItem> = world
        .query_filtered::<(&GridItem, &ItemPlace), (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .filter(|(_, place)| place.container == *id)
        .map(|(item, place)| PlacedItem {
            item: item.clone(),
            place: place.clone(),
        })
        .collect();
    items.sort_by_key(|placed| placed.item.uid);
    items
}

/// An item and where it is.
pub fn placed_item(world: &mut World, uid: ItemUid) -> Option<PlacedItem> {
    let entity = named_entity(world, &item_entity_name(uid))?;
    Some(PlacedItem {
        item: world.get::<GridItem>(entity)?.clone(),
        place: world.get::<ItemPlace>(entity)?.clone(),
    })
}

/// The record key of a container's items.
pub fn container_key(container: &ItemContainer) -> RecordKey {
    RecordKey::new(
        container.record_owner.clone(),
        container.record_part.clone(),
    )
}

/// One saved item (the contract's container record format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SavedItem {
    uid: u64,
    base: String,
    rarity: String,
    affixes: Vec<(String, i32)>,
    level: u32,
    quantity: u32,
    size: [u32; 2],
    x: u32,
    y: u32,
    rotated: bool,
}

/// The saved form of a container's items: `{"items": [...]}`.
pub fn container_record(items: &[PlacedItem]) -> Value {
    let saved: Vec<SavedItem> = items
        .iter()
        .map(|placed| SavedItem {
            uid: placed.item.uid.0,
            base: placed.item.base.clone(),
            rarity: placed.item.rarity.clone(),
            affixes: placed
                .item
                .affixes
                .iter()
                .map(|affix| (affix.affix.clone(), affix.value))
                .collect(),
            level: placed.item.level_requirement,
            quantity: placed.item.quantity,
            size: [placed.item.size.x, placed.item.size.y],
            x: placed.place.x,
            y: placed.place.y,
            rotated: placed.place.rotated,
        })
        .collect();
    json!({ "items": saved })
}

/// The items of a saved container record, placed in `container`.
pub fn items_from_record(data: &Value, container: &ContainerId) -> Result<Vec<PlacedItem>, String> {
    let saved: Vec<SavedItem> = serde_json::from_value(
        data.get("items")
            .cloned()
            .ok_or_else(|| "the record has no `items`".to_string())?,
    )
    .map_err(|error| format!("the items cannot be read: {error}"))?;
    Ok(saved
        .into_iter()
        .map(|saved| PlacedItem {
            item: GridItem {
                uid: ItemUid(saved.uid),
                base: saved.base,
                rarity: saved.rarity,
                affixes: saved
                    .affixes
                    .into_iter()
                    .map(|(affix, value)| crate::RolledAffix { affix, value })
                    .collect(),
                level_requirement: saved.level,
                quantity: saved.quantity,
                size: UVec2::new(saved.size[0], saved.size[1]),
            },
            place: ItemPlace {
                container: container.clone(),
                x: saved.x,
                y: saved.y,
                rotated: saved.rotated,
            },
        })
        .collect())
}

/// The saved items of `container` (none when there are no records or no record yet). A record that
/// cannot be read is an error naming the container: the caller must not overwrite it.
pub fn load_container_items(
    world: &World,
    container: &ItemContainer,
) -> Result<Vec<PlacedItem>, String> {
    let Some(store) = world.get_resource::<RecordStore>() else {
        return Ok(Vec::new());
    };
    let key = container_key(container);
    let record = store
        .read(&key)
        .map_err(|error| format!("{}: {error}", container.id.0))?;
    let Some(record) = record else {
        return Ok(Vec::new());
    };
    record
        .check_not_newer_than(&key, CONTAINER_RECORD_VERSION)
        .map_err(|error| format!("{}: {error}", container.id.0))?;
    items_from_record(&record.data, &container.id)
        .map_err(|error| format!("{}: {error}", container.id.0))
}

/// Spawns a container entity (private to `player`).
pub fn spawn_container(world: &mut World, container: ItemContainer, player: PlayerId) -> Entity {
    let name = container_entity_name(&container.id);
    world
        .spawn((
            Name::new(name),
            container,
            NetPrivateTo {
                client_id: player.0,
            },
        ))
        .id()
}

/// Spawns an item entity (private to `player`).
pub fn spawn_item(world: &mut World, placed: PlacedItem, player: PlayerId) -> Entity {
    world
        .spawn((
            Name::new(item_entity_name(placed.item.uid)),
            placed.item,
            placed.place,
            NetPrivateTo {
                client_id: player.0,
            },
        ))
        .id()
}

/// Removes a container and its items from the world (their records stay).
pub fn remove_container(world: &mut World, id: &ContainerId) {
    let uids: Vec<ItemUid> = container_items(world, id)
        .into_iter()
        .map(|placed| placed.item.uid)
        .collect();
    for uid in uids {
        if let Some(entity) = named_entity(world, &item_entity_name(uid)) {
            world.entity_mut(entity).insert(RemoveNetEntity);
        }
    }
    if let Some(entity) = container_entity(world, id) {
        world.entity_mut(entity).insert(RemoveNetEntity);
    }
}

/// Keeps every container, its items and every wallet private to the player whose body plays their
/// character (a takeover changes the player). Item modules call it every tick on the server.
pub fn sync_item_privacy(world: &mut World) {
    let owners: BTreeMap<CharacterId, u32> = world
        .query_filtered::<(&PlayerId, &PlayingCharacter), (Without<RetiredEntity>, Without<RemovePhysicsBody>)>()
        .iter(world)
        .map(|(player, playing)| (playing.id.clone(), player.0))
        .collect();
    let containers: BTreeMap<ContainerId, CharacterId> = world
        .query_filtered::<&ItemContainer, (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .map(|container| (container.id.clone(), container.character.clone()))
        .collect();
    let mut changes = Vec::new();
    let mut entities = world.query_filtered::<(
        Entity,
        Option<&ItemContainer>,
        Option<&ItemPlace>,
        Option<&crate::Wallet>,
        Option<&NetPrivateTo>,
    ), (Without<RetiredEntity>, Without<RemoveNetEntity>)>();
    for (entity, container, place, wallet, private) in entities.iter(world) {
        let character = match (container, place, wallet) {
            (Some(container), _, _) => Some(&container.character),
            (_, Some(place), _) => containers.get(&place.container),
            (_, _, Some(wallet)) => Some(&wallet.character),
            _ => None,
        };
        let Some(client_id) = character.and_then(|character| owners.get(character)) else {
            continue;
        };
        if private.map(|p| p.client_id) != Some(*client_id) {
            changes.push((entity, *client_id));
        }
    }
    for (entity, client_id) in changes {
        world.entity_mut(entity).insert(NetPrivateTo { client_id });
    }
}

/// Shows `text` to the player of `body` (its [`ItemNotice`]).
pub fn show_item_notice(world: &mut World, body: Entity, text: impl Into<String>) {
    let serial = world
        .get::<ItemNotice>(body)
        .map_or(0, |notice| notice.serial)
        + 1;
    world.entity_mut(body).insert(ItemNotice {
        text: text.into(),
        serial,
    });
}

/// Clears the player's item notice after a request that succeeded (an old refusal no longer
/// applies).
pub fn clear_item_notice(world: &mut World, body: Entity) {
    if world
        .get::<ItemNotice>(body)
        .is_some_and(|notice| !notice.text.is_empty())
    {
        let serial = world
            .get::<ItemNotice>(body)
            .map_or(0, |notice| notice.serial)
            + 1;
        world.entity_mut(body).insert(ItemNotice {
            text: String::new(),
            serial,
        });
    }
}

/// Hands out `count` new item uids (see the module docs). Fails only when the reservation cannot be
/// saved.
pub fn allocate_item_uids(world: &mut World, count: u64) -> Result<Vec<ItemUid>, String> {
    #[derive(Resource, Default)]
    struct ItemUidCounter {
        next: u64,
        reserved_until: u64,
    }
    let mut counter = world
        .remove_resource::<ItemUidCounter>()
        .unwrap_or_default();
    if counter.next == 0 {
        counter.next = 1;
    }
    if counter.next + count > counter.reserved_until {
        let key = RecordKey::new(ITEM_UID_RECORD_OWNER, ITEM_UID_RECORD_PART);
        if let Some(mut store) = world.get_resource_mut::<RecordStore>() {
            let saved = store
                .read(&key)
                .map_err(|error| format!("the item id counter cannot be read: {error}"))?
                .and_then(|record| record.data.get("reserved_until").and_then(Value::as_u64))
                .unwrap_or(1);
            let start = counter.next.max(saved);
            let until = start + count.max(ITEM_UID_BLOCK);
            if let Err(error) = store.put(key, 1, json!({ "reserved_until": until })) {
                world.insert_resource(counter);
                return Err(format!("the item id counter cannot be saved: {error}"));
            }
            counter.next = start;
            counter.reserved_until = until;
        } else {
            counter.reserved_until = counter.next + count.max(ITEM_UID_BLOCK);
        }
    }
    let uids = (counter.next..counter.next + count).map(ItemUid).collect();
    counter.next += count;
    world.insert_resource(counter);
    Ok(uids)
}

/// A set of item changes that happens all at once (see the module docs).
#[derive(Debug, Clone, Default)]
pub struct ItemTransaction {
    /// New items, or new values of existing ones (by uid), with their place.
    pub put: Vec<PlacedItem>,
    /// Items that stop existing (merged into a stack, deposited).
    pub remove: Vec<ItemUid>,
    /// More records written in the same batch (e.g. a balance), with their data version.
    pub records: Vec<(RecordKey, u32, Value)>,
}

/// Applies `transaction` for `player`: saves every touched container (and the extra records) in one
/// atomic batch, then changes the world. Nothing changes when the save fails. New items are private
/// to `player`.
pub fn apply_item_transaction(
    world: &mut World,
    player: PlayerId,
    transaction: ItemTransaction,
) -> Result<(), String> {
    // Containers touched: where changed items are now and where they go.
    let mut touched: Vec<ContainerId> = Vec::new();
    let mut touch = |id: &ContainerId| {
        if !touched.contains(id) {
            touched.push(id.clone());
        }
    };
    let changed: Vec<ItemUid> = transaction
        .put
        .iter()
        .map(|placed| placed.item.uid)
        .chain(transaction.remove.iter().copied())
        .collect();
    for uid in &changed {
        if let Some(placed) = placed_item(world, *uid) {
            touch(&placed.place.container);
        }
    }
    for placed in &transaction.put {
        touch(&placed.place.container);
    }
    let mut batch = RecordBatch::new();
    for id in &touched {
        let container =
            container_of(world, id).ok_or_else(|| format!("container {} is not open", id.0))?;
        let mut items: Vec<PlacedItem> = container_items(world, id)
            .into_iter()
            .filter(|placed| !changed.contains(&placed.item.uid))
            .collect();
        items.extend(
            transaction
                .put
                .iter()
                .filter(|placed| placed.place.container == *id)
                .cloned(),
        );
        items.sort_by_key(|placed| placed.item.uid);
        batch = batch.put(
            container_key(&container),
            CONTAINER_RECORD_VERSION,
            container_record(&items),
        );
    }
    for (key, version, data) in transaction.records {
        batch = batch.put(key, version, data);
    }
    if let Some(mut store) = world.get_resource_mut::<RecordStore>()
        && !batch.is_empty()
    {
        store
            .write(batch)
            .map_err(|error| format!("the items could not be saved, nothing changed: {error}"))?;
    }
    for uid in &transaction.remove {
        if let Some(entity) = named_entity(world, &item_entity_name(*uid)) {
            world.entity_mut(entity).insert(RemoveNetEntity);
        }
    }
    for placed in transaction.put {
        match named_entity(world, &item_entity_name(placed.item.uid)) {
            Some(entity) => {
                if world.get::<GridItem>(entity) != Some(&placed.item) {
                    world.entity_mut(entity).insert(placed.item);
                }
                if world.get::<ItemPlace>(entity) != Some(&placed.place) {
                    world.entity_mut(entity).insert(placed.place);
                }
            }
            None => {
                spawn_item(world, placed, player);
            }
        }
    }
    Ok(())
}

/// Moves item `uid` to `(x, y, rotated)` of container `to`, for `player`: the one contract
/// operation for moving items between containers (bag, equipment, chest), whatever module owns
/// them. Checks ownership, both containers' rules ([`plan_move`]) and saves atomically. The error is
/// for the player.
pub fn move_item(
    world: &mut World,
    player: PlayerId,
    uid: ItemUid,
    to: &ContainerId,
    x: u32,
    y: u32,
    rotated: bool,
) -> Result<(), String> {
    let (body, playing) = playing_body(world, player).ok_or("you are not playing a character")?;
    let moving = placed_item(world, uid).ok_or_else(|| format!("there is no item {}", uid.0))?;
    let from = container_of(world, &moving.place.container)
        .ok_or("that item is not in an open container")?;
    let target = container_of(world, to).ok_or_else(|| format!("{} is not open", to.0))?;
    if from.character != playing.id || target.character != playing.id {
        return Err("that is not your container".to_string());
    }
    let to_items = container_items(world, to);
    let catalog = world.resource::<GridItemCatalog>().clone();
    let context = MoveContext {
        catalog: &catalog,
        level: character_level(world, body),
        in_safe_area: in_safe_area(world, body),
    };
    let outcome = plan_move(
        &context,
        &moving.item,
        &from,
        &target,
        &to_items,
        x,
        y,
        rotated,
    )?;
    let transaction = match outcome {
        MoveOutcome::Placed(place) => ItemTransaction {
            put: vec![PlacedItem {
                item: moving.item,
                place,
            }],
            ..Default::default()
        },
        MoveOutcome::Stacked { into, amount } => {
            let mut stack = to_items
                .into_iter()
                .find(|placed| placed.item.uid == into)
                .expect("planned onto an item of the target");
            stack.item.quantity += amount;
            let mut rest = moving;
            rest.item.quantity -= amount;
            if rest.item.quantity == 0 {
                ItemTransaction {
                    put: vec![stack],
                    remove: vec![rest.item.uid],
                    ..Default::default()
                }
            } else {
                ItemTransaction {
                    put: vec![stack, rest],
                    ..Default::default()
                }
            }
        }
    };
    apply_item_transaction(world, player, transaction)
}

/// Whether this machine decides the game (a server, or a game without a network).
pub fn is_authority(world: &World) -> bool {
    !world
        .resource::<retreat_engine_api::NetInputQueue>()
        .accepting
}

/// Every live body that plays a saved character: its player and character, by character id.
pub fn playing_characters(world: &mut World) -> Vec<(PlayerId, PlayingCharacter)> {
    let mut found: Vec<(PlayerId, PlayingCharacter)> = world
        .query_filtered::<(&PlayerId, &PlayingCharacter), (
            Without<RetiredEntity>,
            Without<RemovePhysicsBody>,
        )>()
        .iter(world)
        .map(|(player, playing)| (*player, playing.clone()))
        .collect();
    found.sort_by(|a, b| a.1.id.cmp(&b.1.id));
    found
}

/// Opens a container: spawns it and its saved items for `player`. A record that cannot be read
/// leaves it closed (logged), so it is never overwritten with nothing.
pub fn open_container(
    world: &mut World,
    container: ItemContainer,
    player: PlayerId,
) -> Result<(), String> {
    let items = load_container_items(world, &container)?;
    spawn_container(world, container, player);
    for placed in items {
        spawn_item(world, placed, player);
    }
    Ok(())
}

/// On the authoritative machine, every tick: every playing character has its container of `kind`
/// (made by `make`, filled from its record), and containers of `kind` whose character stopped
/// playing are removed (their records stay). Item modules call it for their own kind.
pub fn keep_character_containers(
    world: &mut World,
    kind: crate::ContainerKind,
    make: impl Fn(&PlayingCharacter) -> ItemContainer,
) {
    if !is_authority(world) {
        return;
    }
    close_orphan_containers(world, kind);
    for (player, playing) in playing_characters(world) {
        let id = crate::container_id(kind, &playing.id);
        if container_entity(world, &id).is_some() {
            continue;
        }
        if let Err(problem) = open_container(world, make(&playing), player) {
            error!(
                "{} of {} cannot be opened: {problem}",
                kind.word(),
                playing.id.0
            );
        }
    }
}

/// Removes containers of `kind` whose character no longer plays (authoritative machine only).
pub fn close_orphan_containers(world: &mut World, kind: crate::ContainerKind) {
    if !is_authority(world) {
        return;
    }
    let playing: Vec<CharacterId> = playing_characters(world)
        .into_iter()
        .map(|(_, playing)| playing.id)
        .collect();
    let orphans: Vec<ContainerId> = world
        .query_filtered::<&ItemContainer, (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .filter(|container| container.kind == kind && !playing.contains(&container.character))
        .map(|container| container.id.clone())
        .collect();
    for id in orphans {
        remove_container(world, &id);
    }
}

/// The whole of a `<module>.put <uid> <x> <y> <0|1>` request (a drop on a container of `kind` of
/// the sender's character): routed like every player request, then [`move_item`]; a refusal is
/// the player's notice. `command` is the command id, to send the line on.
pub fn put_request(
    world: &mut World,
    args: &retreat_engine_api::CommandArgs,
    command: &str,
    kind: crate::ContainerKind,
) -> retreat_engine_api::CommandResult {
    args.expect_count(4)?;
    let uid: u64 = args.parse(0, "item uid")?;
    let x: u32 = args.parse(1, "x")?;
    let y: u32 = args.parse(2, "y")?;
    let rotated: u8 = args.parse(3, "rotated (0 or 1)")?;
    if rotated > 1 {
        return Err(args.failure("rotated must be 0 or 1"));
    }
    if !crate::route_request(world, format!("{command} {uid} {x} {y} {rotated}")) {
        return Ok(retreat_engine_api::CommandOutcome::Continue);
    }
    let player = crate::actor(world, args)?;
    let Some((body, playing)) = playing_body(world, player) else {
        return crate::refused(world, args, "you are not playing a character");
    };
    let to = crate::container_id(kind, &playing.id);
    match move_item(world, player, ItemUid(uid), &to, x, y, rotated == 1) {
        Ok(()) => {
            clear_item_notice(world, body);
            Ok(retreat_engine_api::CommandOutcome::Continue)
        }
        Err(why) => {
            show_item_notice(world, body, why.clone());
            Ok(retreat_engine_api::CommandOutcome::Continue)
        }
    }
}

/// What [`add_to_container`] puts in.
#[derive(Debug, Clone, Default)]
pub struct NewItems {
    /// Items that do not stack, each to the first free spot (their uid is replaced by a new one).
    pub unique: Vec<GridItem>,
    /// An amount of a stackable base (`base id`, quantity): onto its stacks, then new stacks.
    pub stack: Option<(String, u32)>,
}

/// Puts `new` into grid container `to` for `player`, with new uids, plus `records` in the same
/// atomic batch (a balance). Fails, changing nothing, when not everything fits. Returns the uids
/// of the new items.
pub fn add_to_container(
    world: &mut World,
    player: PlayerId,
    to: &ContainerId,
    new: NewItems,
    records: Vec<(RecordKey, u32, Value)>,
) -> Result<Vec<ItemUid>, String> {
    let container = container_of(world, to).ok_or_else(|| format!("{} is not open", to.0))?;
    let crate::ContainerLayout::Grid { columns, rows, .. } = container.layout else {
        return Err(format!(
            "the {} takes no new items",
            container.title.to_lowercase()
        ));
    };
    let catalog = world.resource::<GridItemCatalog>().clone();
    let mut items = container_items(world, to);
    let mut put: Vec<PlacedItem> = Vec::new();
    // Places first (with placeholder uids), uids once everything fits.
    let mut unique_places = Vec::new();
    for (index, item) in new.unique.iter().enumerate() {
        let (at, rotated) = crate::first_free_spot(columns, rows, &items, item.size)
            .ok_or_else(|| format!("the {} is full", container.title.to_lowercase()))?;
        let place = ItemPlace {
            container: to.clone(),
            x: at.x,
            y: at.y,
            rotated,
        };
        let mut taken = item.clone();
        taken.uid = ItemUid(u64::MAX - index as u64);
        items.push(PlacedItem {
            item: taken,
            place: place.clone(),
        });
        unique_places.push(place);
    }
    let mut new_stacks = Vec::new();
    if let Some((base_id, quantity)) = &new.stack {
        let base = catalog
            .base(base_id)
            .ok_or_else(|| format!("unknown item `{base_id}`"))?;
        let (grown, stacks) = crate::plan_stack_add(
            columns,
            rows,
            &items,
            base_id,
            base.size,
            base.max_stack,
            *quantity,
        )
        .ok_or_else(|| format!("the {} is full", container.title.to_lowercase()))?;
        put.extend(grown);
        new_stacks = stacks;
    }
    let uids = allocate_item_uids(world, (unique_places.len() + new_stacks.len()) as u64)?;
    let mut fresh = uids.iter().copied();
    for (item, place) in new.unique.into_iter().zip(unique_places) {
        put.push(PlacedItem {
            item: GridItem {
                uid: fresh.next().expect("one uid each"),
                ..item
            },
            place,
        });
    }
    if let Some((base_id, _)) = new.stack {
        let base = catalog.base(&base_id).expect("checked above");
        let rarity = catalog
            .rarities
            .first()
            .map_or(String::new(), |rarity| rarity.id.clone());
        for stack in new_stacks {
            put.push(PlacedItem {
                item: GridItem {
                    uid: fresh.next().expect("one uid each"),
                    base: base_id.clone(),
                    rarity: rarity.clone(),
                    affixes: Vec::new(),
                    level_requirement: 0,
                    quantity: stack.quantity,
                    size: base.size,
                },
                place: ItemPlace {
                    container: to.clone(),
                    x: stack.at.x,
                    y: stack.at.y,
                    rotated: stack.rotated,
                },
            });
        }
    }
    apply_item_transaction(
        world,
        player,
        ItemTransaction {
            put,
            remove: Vec::new(),
            records,
        },
    )?;
    Ok(uids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RolledAffix;

    #[test]
    fn the_record_format_keeps_everything_of_an_item() {
        let container = ContainerId("bag/ada-1".to_string());
        let items = vec![PlacedItem {
            item: GridItem {
                uid: ItemUid(42),
                base: "short_sword".to_string(),
                rarity: "rare".to_string(),
                affixes: vec![RolledAffix {
                    affix: "strength".to_string(),
                    value: 7,
                }],
                level_requirement: 4,
                quantity: 1,
                size: UVec2::new(1, 3),
            },
            place: ItemPlace {
                container: container.clone(),
                x: 2,
                y: 1,
                rotated: true,
            },
        }];
        let record = container_record(&items);
        assert_eq!(items_from_record(&record, &container), Ok(items));
        assert!(items_from_record(&json!({}), &container).is_err());
    }
}
