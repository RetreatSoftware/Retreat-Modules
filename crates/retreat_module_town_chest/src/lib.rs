//! Town chest (crawler plan, phase 2): safe storage that is never lost, usable only in the city at a
//! chest spot (the map's `ServiceSpots` of kind `town_chest`), with unlimited capacity (decided with
//! the user): a grid of [`TownChestConfig::columns`] whose rows grow so there are always
//! [`TownChestConfig::spare_rows`] empty rows below the lowest item (at least
//! [`TownChestConfig::min_rows`]); a window shows [`TownChestConfig::visible_rows`] and scrolls.
//!
//! **One chest per account, in every world** (decided with the user 2026-11-28): every character
//! of an account, in any world, uses the same chest. It is saved in this module's record part
//! `town_chest` of `account/<id>` (no world in the key), on every change, in the contract's atomic
//! item transaction. Every world server shares the records, so two characters of one account in
//! two worlds may have the chest open at once: each transaction writes only if the chest's record
//! is still what that server last saw (else nothing moves and the player is told to look again),
//! and an open chest is read again every [`TownChestConfig::refresh_seconds`], so each window
//! shows what the other world did. In one world an account plays one character at a time; a chest
//! that is already open for another character of the account there is not opened twice.
//!
//! **Open while used.** `chest.open` (a player request, in a safe area within reach of a chest
//! spot) opens the contract container `chest/<character>` with the account's saved items, private
//! to the player; it closes on `chest.close`, when the character walks out of reach or leaves the
//! safe area, and when the character stops playing. While closed its items exist only in the
//! record, so a big chest costs nothing. Items move in and out with the bag's and the chest's drops
//! (`chest.put <uid> <x> <y> <0|1>`); the container is marked city-only, so no move touches it
//! outside a safe area.
//!
//! A "Town chest" panel (Open / Close) shows while this machine's character stands at a chest spot.
//! Assertions: `assert.chest_open`, `assert.chest_closed`, `assert.chest_items <count>`.

use bevy::prelude::*;
use retreat_engine_api::RecordStore;
use retreat_engine_api::{
    AppCommandExt, AppNetExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult, NetLocalClient,
    PanelAnchor, PanelDirection, UiButtonEntry, UiPanel, UiPanels,
};
use retreat_gameplay_api::{
    CharacterSize, ContainerKind, ContainerLayout, GameplaySystems, ItemContainer, PlacedItem,
    PlayingCharacter, ServiceSpots, SoloPlayer, account_record_owner, actor, add_gameplay_contract,
    close_orphan_containers, container_entity, container_id, container_items, container_key,
    in_safe_area, is_authority, local_player_id, placed_size, playing_body, playing_characters,
    put_request, read_container, refresh_container_from_record, refused, remove_container,
    route_request, show_item_notice, spawn_container, spawn_item, sync_item_privacy,
    ticks_from_seconds,
};

/// This module's record part (and id).
pub const CHEST_RECORD_PART: &str = "town_chest";
/// The `ServiceSpots` kind of a chest spot.
pub const CHEST_SPOT_KIND: &str = "town_chest";

/// Chest numbers (placeholders; insert before adding the plugin to change).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TownChestConfig {
    pub columns: u32,
    /// The fewest rows, however empty.
    pub min_rows: u32,
    /// Empty rows kept below the lowest item (room to put more: the chest has no limit).
    pub spare_rows: u32,
    /// Rows a window shows at once.
    pub visible_rows: u32,
    /// How often an open chest is read again, so it shows what the account's characters in other
    /// worlds put in or took out.
    pub refresh_seconds: f32,
}

impl Default for TownChestConfig {
    fn default() -> Self {
        Self {
            columns: 10,
            min_rows: 8,
            spare_rows: 2,
            visible_rows: 6,
            refresh_seconds: 0.5,
        }
    }
}

/// The rows a chest holding `items` has: the lowest item's bottom plus the spare rows, at least the
/// minimum. Pure.
pub fn chest_rows(config: &TownChestConfig, items: &[PlacedItem]) -> u32 {
    let lowest = items
        .iter()
        .map(|placed| placed.place.y + placed_size(placed.item.size, placed.place.rotated).y)
        .max()
        .unwrap_or(0);
    (lowest + config.spare_rows).max(config.min_rows)
}

const PANEL: &str = "town_chest";
const PANEL_ENTRIES: &[UiButtonEntry] = &[
    UiButtonEntry {
        id: "open",
        label: "Open chest",
        command_line: "chest.open",
    },
    UiButtonEntry {
        id: "close",
        label: "Close chest",
        command_line: "chest.close",
    },
];

/// Adds the town chest and the `chest.*` commands.
pub struct TownChestModulePlugin;

impl Plugin for TownChestModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<TownChestConfig>();
        let config = app.world().resource::<TownChestConfig>().clone();
        assert!(
            config.columns > 0 && config.min_rows > 0 && config.visible_rows > 0,
            "invalid TownChestConfig: columns, min_rows and visible_rows must be at least 1"
        );
        app.register_command("chest.open", "chest.open", open_command)
            .allow_client_command("chest.open")
            .register_command("chest.close", "chest.close", close_command)
            .allow_client_command("chest.close")
            .register_command("chest.put", "chest.put <uid> <x> <y> <0|1>", put_command)
            .allow_client_command("chest.put")
            .register_command(
                "assert.chest_open",
                "assert.chest_open",
                assert_open_command,
            )
            .register_command(
                "assert.chest_closed",
                "assert.chest_closed",
                assert_closed_command,
            )
            .register_command(
                "assert.chest_items",
                "assert.chest_items <count>",
                assert_items_command,
            )
            .register_panel(UiPanel {
                id: PANEL,
                title: Some("Town chest"),
                anchor: PanelAnchor::TopRight,
                direction: PanelDirection::Column,
                entries: PANEL_ENTRIES,
                visible: false,
            })
            .add_systems(FixedUpdate, keep_chests.in_set(GameplaySystems::Items))
            .add_systems(Update, show_panel_at_chest);
    }
}

/// The chest container of `playing` with `rows` rows: the account's chest, shown for this
/// character.
pub fn chest_container(
    config: &TownChestConfig,
    playing: &PlayingCharacter,
    rows: u32,
) -> ItemContainer {
    ItemContainer {
        id: container_id(ContainerKind::Chest, &playing.id),
        kind: ContainerKind::Chest,
        title: "Town chest".to_string(),
        layout: ContainerLayout::Grid {
            columns: config.columns,
            rows,
            visible_rows: config.visible_rows,
        },
        character: playing.id.clone(),
        only_in_safe_area: true,
        drop_command: "chest.put".to_string(),
        record_owner: account_record_owner(&playing.account),
        record_part: CHEST_RECORD_PART.to_string(),
    }
}

/// Whether `body` may use a chest now: in a safe area and within reach of a chest spot.
fn at_chest(world: &World, body: Entity) -> bool {
    let Some(transform) = world.get::<Transform>(body) else {
        return false;
    };
    let feet = world
        .resource::<CharacterSize>()
        .feet(transform.translation);
    in_safe_area(world, body) && world.resource::<ServiceSpots>().near(CHEST_SPOT_KIND, feet)
}

/// Opens or closes as the rules say, reads open chests again now and then (other worlds), and
/// grows or shrinks open chests' rows.
fn keep_chests(world: &mut World, mut ticks: Local<u64>) {
    if !is_authority(world) {
        return;
    }
    close_orphan_containers(world, ContainerKind::Chest);
    let config = world.resource::<TownChestConfig>().clone();
    let timestep = world.resource::<Time<Fixed>>().timestep();
    let every = ticks_from_seconds(config.refresh_seconds, timestep).max(1);
    *ticks += 1;
    let refresh = (*ticks).is_multiple_of(u64::from(every));
    for (player, playing) in playing_characters(world) {
        let id = container_id(ContainerKind::Chest, &playing.id);
        let Some(entity) = container_entity(world, &id) else {
            continue;
        };
        let Some((body, _)) = playing_body(world, player) else {
            continue;
        };
        if !at_chest(world, body) {
            remove_container(world, &id);
            continue;
        }
        if refresh && let Err(problem) = refresh_container_from_record(world, &id) {
            error!(
                "the town chest of {} cannot be read again: {problem}",
                playing.account.0
            );
        }
        let rows = chest_rows(&config, &container_items(world, &id));
        let wanted = chest_container(&config, &playing, rows);
        if world.get::<ItemContainer>(entity) != Some(&wanted) {
            world.entity_mut(entity).insert(wanted);
        }
    }
    sync_item_privacy(world);
}

/// `chest.open`.
fn open_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    if !route_request(world, "chest.open".to_string()) {
        return Ok(CommandOutcome::Continue);
    }
    let player = actor(world, args)?;
    let Some((body, playing)) = playing_body(world, player) else {
        return refused(world, args, "you are not playing a character");
    };
    let id = container_id(ContainerKind::Chest, &playing.id);
    if container_entity(world, &id).is_some() {
        return Ok(CommandOutcome::Continue);
    }
    if !at_chest(world, body) {
        let why = "walk up to a town chest in the city to open it";
        show_item_notice(world, body, why);
        return Ok(CommandOutcome::Continue);
    }
    let config = world.resource::<TownChestConfig>().clone();
    let probe = chest_container(&config, &playing, config.min_rows);
    if let Some(other) = open_elsewhere_here(world, &probe) {
        let why = format!("your town chest is open for {other}; close it there first");
        show_item_notice(world, body, why);
        return Ok(CommandOutcome::Continue);
    }
    let (seen, items) = match read_container(world, &probe) {
        Ok(read) => read,
        Err(problem) => {
            error!("town chest of {} cannot be read: {problem}", playing.id.0);
            let why = "the chest cannot be opened now (its record cannot be read)";
            show_item_notice(world, body, why);
            return Ok(CommandOutcome::Continue);
        }
    };
    let rows = chest_rows(&config, &items);
    spawn_container(
        world,
        chest_container(&config, &playing, rows),
        player,
        seen,
    );
    for placed in items {
        spawn_item(world, placed, player);
    }
    Ok(CommandOutcome::Continue)
}

/// The character for whom this machine already shows the same chest record as `chest`, if any
/// (one item must never be in the world twice). A closed chest stays a while as a removed or
/// retired entity; it does not count.
fn open_elsewhere_here(world: &mut World, chest: &ItemContainer) -> Option<String> {
    if !world.contains_resource::<RecordStore>() {
        return None;
    }
    let key = container_key(chest);
    world
        .query_filtered::<&ItemContainer, (
            Without<retreat_engine_api::RemoveNetEntity>,
            Without<retreat_engine_api::RetiredEntity>,
        )>()
        .iter(world)
        .find(|open| {
            open.kind == ContainerKind::Chest && open.id != chest.id && container_key(open) == key
        })
        .map(|open| open.character.0.clone())
}

/// `chest.close`.
fn close_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    if !route_request(world, "chest.close".to_string()) {
        return Ok(CommandOutcome::Continue);
    }
    let player = actor(world, args)?;
    if let Some((_, playing)) = playing_body(world, player) {
        remove_container(world, &container_id(ContainerKind::Chest, &playing.id));
    }
    Ok(CommandOutcome::Continue)
}

/// `chest.put <uid> <x> <y> <0|1>`.
fn put_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    put_request(world, args, "chest.put", ContainerKind::Chest)
}

/// The chest id of this machine's player, if it plays.
fn own_chest(world: &mut World) -> Option<(Entity, retreat_gameplay_api::ContainerId)> {
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )?;
    let (body, playing) = playing_body(world, player)?;
    Some((body, container_id(ContainerKind::Chest, &playing.id)))
}

fn assert_open_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (_, id) =
        own_chest(world).ok_or_else(|| args.failure("this machine's player is not playing"))?;
    if container_entity(world, &id).is_some() {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure("the town chest is closed"))
    }
}

fn assert_closed_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (_, id) =
        own_chest(world).ok_or_else(|| args.failure("this machine's player is not playing"))?;
    if container_entity(world, &id).is_none() {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure("the town chest is open"))
    }
}

/// `assert.chest_items <count>` (the chest must be open).
fn assert_items_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "count")?;
    let (_, id) =
        own_chest(world).ok_or_else(|| args.failure("this machine's player is not playing"))?;
    if container_entity(world, &id).is_none() {
        return Err(args.failure("the town chest is closed; open it first"));
    }
    let count = container_items(world, &id).len();
    if count == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "the chest holds {count} items, expected {expected}"
        )))
    }
}

/// Shows the panel while this machine's character stands at a chest: Open when closed, Close when
/// open.
fn show_panel_at_chest(world: &mut World) {
    let state = own_chest(world).map(|(body, id)| {
        let near = world
            .get::<Transform>(body)
            .map(|transform| {
                world
                    .resource::<CharacterSize>()
                    .feet(transform.translation)
            })
            .is_some_and(|feet| world.resource::<ServiceSpots>().near(CHEST_SPOT_KIND, feet));
        (near, container_entity(world, &id).is_some())
    });
    let (visible, open) = match state {
        Some((near, open)) => (near || open, open),
        None => (false, false),
    };
    let mut panels = world.resource_mut::<UiPanels>();
    if panels.is_visible(PANEL) != visible {
        panels.set_visible(PANEL, visible).expect("registered");
    }
    // Exactly one button: Open while closed, Close while open.
    for (entry, wanted) in [("open", !open), ("close", open)] {
        let shown = panels
            .visible_entries(PANEL)
            .iter()
            .any(|visible| visible.id == entry);
        if shown != wanted {
            if wanted {
                panels.show_entry(PANEL, entry).expect("registered");
            } else {
                panels.hide_entry(PANEL, entry).expect("registered");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use retreat_gameplay_api::{ContainerId, GridItem, ItemPlace, ItemUid};

    fn at(y: u32, height: u32, rotated: bool) -> PlacedItem {
        PlacedItem {
            item: GridItem {
                uid: ItemUid(1),
                base: "x".to_string(),
                rarity: "common".to_string(),
                affixes: Vec::new(),
                level_requirement: 1,
                quantity: 1,
                size: UVec2::new(1, height),
            },
            place: ItemPlace {
                container: ContainerId("chest/a-1".to_string()),
                x: 0,
                y,
                rotated,
            },
        }
    }

    #[test]
    fn rows_keep_spare_room_below_the_lowest_item_and_never_fall_below_the_minimum() {
        let config = TownChestConfig::default();
        assert_eq!(chest_rows(&config, &[]), 8);
        assert_eq!(chest_rows(&config, &[at(2, 2, false)]), 8);
        assert_eq!(chest_rows(&config, &[at(10, 3, false)]), 15);
        assert_eq!(chest_rows(&config, &[at(10, 3, true)]), 13);
    }
}
