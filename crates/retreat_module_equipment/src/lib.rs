//! Equipment (crawler plan, phase 2): every playing character's worn items, one per slot: head,
//! chest, hands, legs, feet, main hand, off hand (decided with the user; a two-handed weapon takes
//! the main hand and needs the off hand empty). An item goes only in a slot its base lists and only
//! when the character's level reaches its requirement (the contract's `check_equip`).
//!
//! The equipment is the contract container `equipment/<character>` (layout `Equipment`: an item's
//! place `x` is its slot index), opened with its saved items on the authoritative machine while the
//! character plays; private to the owning player; saved in this module's record part `equipment` of
//! `character/<id>` with every change. Unequipping is a move to the bag (the bag's drop).
//!
//! Commands: `equipment.put <uid> <slot index> <y> <0|1>` (the equipment's drop; a player request
//! routed to the server). Test tools while levels do not exist yet (progression is phase 3):
//! `equipment.test_level <level>` and `equipment.test_level_up` set the character's
//! `CharacterLevel` (not saved), clients may send them and a "Level (test)" panel shows only when
//! [`EquipmentConfig::test_commands`] is on. Assertion: `assert.equipped <slot> <base|nothing>`.
//!
//! Stats from gear come with the stat list (phase 3, still open with the user); today an item's
//! affixes only show in its tooltip.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppNetExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult, NetLocalClient,
    PanelAnchor, PanelDirection, UiButtonEntry, UiPanel, UiPanels,
};
use retreat_gameplay_api::{
    CharacterId, CharacterLevel, ContainerKind, ContainerLayout, EquipSlot, GameplaySystems,
    ItemContainer, PlayingCharacter, SoloPlayer, actor, add_gameplay_contract, character_level,
    character_record_owner, container_id, container_items, keep_character_containers,
    local_player_id, playing_body, put_request, refused, route_request, sync_item_privacy,
};

/// This module's record part (and id).
pub const EQUIPMENT_RECORD_PART: &str = "equipment";

/// Switches of equipment (insert before adding the plugin to change).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EquipmentConfig {
    /// The test level commands from clients and the test panel (off by default).
    pub test_commands: bool,
}

const TEST_PANEL: &str = "equipment_test";
const TEST_ENTRIES: &[UiButtonEntry] = &[
    UiButtonEntry {
        id: "level_up",
        label: "Level up (test)",
        command_line: "equipment.test_level_up",
    },
    UiButtonEntry {
        id: "level_reset",
        label: "Back to level 1",
        command_line: "equipment.test_level 1",
    },
];

/// Adds equipment and the `equipment.*` commands.
pub struct EquipmentModulePlugin;

impl Plugin for EquipmentModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<EquipmentConfig>();
        let config = app.world().resource::<EquipmentConfig>().clone();
        app.register_command(
            "equipment.put",
            "equipment.put <uid> <slot index> <y> <0|1>",
            put_command,
        )
        .allow_client_command("equipment.put")
        .register_command(
            "equipment.test_level",
            "equipment.test_level <level>",
            test_level_command,
        )
        .register_command(
            "equipment.test_level_up",
            "equipment.test_level_up",
            test_level_up_command,
        )
        .register_command(
            "assert.equipped",
            "assert.equipped <slot> <base|nothing>",
            assert_equipped_command,
        )
        .register_panel(UiPanel {
            id: TEST_PANEL,
            title: Some("Level (test)"),
            anchor: PanelAnchor::TopLeft,
            direction: PanelDirection::Column,
            entries: TEST_ENTRIES,
            visible: false,
        })
        .add_systems(FixedUpdate, keep_equipment.in_set(GameplaySystems::Items))
        .add_systems(Update, show_test_panel_while_playing);
        if config.test_commands {
            app.allow_client_command("equipment.test_level")
                .allow_client_command("equipment.test_level_up");
        }
    }
}

/// The equipment of `character`.
pub fn equipment_container(character: &CharacterId) -> ItemContainer {
    ItemContainer {
        id: container_id(ContainerKind::Equipment, character),
        kind: ContainerKind::Equipment,
        title: "Equipment".to_string(),
        layout: ContainerLayout::Equipment,
        character: character.clone(),
        only_in_safe_area: false,
        drop_command: "equipment.put".to_string(),
        record_owner: character_record_owner(character),
        record_part: EQUIPMENT_RECORD_PART.to_string(),
    }
}

fn keep_equipment(world: &mut World) {
    keep_character_containers(
        world,
        ContainerKind::Equipment,
        |playing: &PlayingCharacter| equipment_container(&playing.id),
    );
    sync_item_privacy(world);
}

/// `equipment.put <uid> <slot index> <y> <0|1>`.
fn put_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    put_request(world, args, "equipment.put", ContainerKind::Equipment)
}

/// Sets the level of the actor's character (a test tool until progression exists).
fn set_level(world: &mut World, args: &CommandArgs, level: impl Fn(u32) -> u32) -> CommandResult {
    let player = actor(world, args)?;
    let Some((body, _)) = playing_body(world, player) else {
        return refused(world, args, "you are not playing a character");
    };
    let new_level = level(character_level(world, body)).clamp(1, 100);
    world.entity_mut(body).insert(CharacterLevel(new_level));
    Ok(CommandOutcome::Continue)
}

/// `equipment.test_level <level>`.
fn test_level_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let level: u32 = args.parse(0, "level")?;
    if !route_request(world, format!("equipment.test_level {level}")) {
        return Ok(CommandOutcome::Continue);
    }
    set_level(world, args, |_| level)
}

/// `equipment.test_level_up`.
fn test_level_up_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    if !route_request(world, "equipment.test_level_up".to_string()) {
        return Ok(CommandOutcome::Continue);
    }
    set_level(world, args, |level| level + 1)
}

/// `assert.equipped <slot> <base|nothing>` for this machine's player; slots are named like
/// `main_hand`.
fn assert_equipped_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let slot_name: String = args.parse(0, "slot")?;
    let expected: String = args.parse(1, "base")?;
    let slot = EquipSlot::ALL
        .into_iter()
        .find(|slot| slot.label().to_lowercase().replace(' ', "_") == slot_name)
        .ok_or_else(|| {
            args.failure(format!(
                "unknown slot `{slot_name}`; slots: head chest hands legs feet main_hand off_hand"
            ))
        })?;
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
    .map_or_else(|| actor(world, args), Ok)?;
    let (_, playing) = playing_body(world, player)
        .ok_or_else(|| args.failure("this machine's player is not playing a character"))?;
    let worn = container_items(world, &container_id(ContainerKind::Equipment, &playing.id))
        .into_iter()
        .find(|placed| placed.place.x == slot.index())
        .map_or("nothing".to_string(), |placed| placed.item.base);
    if worn == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "the {slot_name} slot holds {worn}, expected {expected}"
        )))
    }
}

/// Shows the test panel while this machine's player plays a character (and tests are on).
fn show_test_panel_while_playing(world: &mut World) {
    let wanted = world.resource::<EquipmentConfig>().test_commands
        && local_player_id(
            *world.resource::<NetLocalClient>(),
            *world.resource::<SoloPlayer>(),
        )
        .is_some_and(|player| playing_body(world, player).is_some());
    if world.resource::<UiPanels>().is_visible(TEST_PANEL) != wanted {
        world
            .resource_mut::<UiPanels>()
            .set_visible(TEST_PANEL, wanted)
            .expect("registered");
    }
}
