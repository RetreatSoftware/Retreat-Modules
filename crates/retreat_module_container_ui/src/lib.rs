//! Container UI (crawler plan, phase 2): one grid UI for every item container of this machine's
//! character (bag, equipment, town chest; later corpse, vendor, trade). It draws through the
//! engine's item grid windows: icons, drag and drop with `R` / right button to turn, scrolling,
//! tooltips. It decides nothing about items: a drop runs the container's own drop command
//! (`<drop_command> <uid> <x> <y> <0|1>`), which the owning module routes to the server.
//!
//! **Opening.** `I` (action `container_ui_toggle`), the "Inventory" button or
//! `container_ui.toggle|open|close` show the windows of the character's open containers (equipment
//! left, bag, chest right); opening releases the cursor. The windows open by themselves when a
//! chest opens. They show only what this machine sees (containers are private to their player).
//!
//! **What a window shows** (pure [`container_view`]): items with their base's icon and color (a
//! hint of the rarity color), stack counts; tooltips with the name in the rarity's color, rarity
//! and kind, rolled affixes, stack size, "Two-handed" and the level requirement (red while the
//! character's level is below it); the bag's footer the carried gold and the safe balance; the
//! equipment's footer the level. Equipment slots are laid out by [`equipment_slots`].
//!
//! **Notices** (the contract's `ItemNotice`, e.g. "needs level 5"; `assert.item_notice <text>`): shown at the bottom of the view
//! for [`ContainerUiConfig::notice_seconds`].
//!
//! Render-only: nothing here changes the simulation.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppHudExt, AppInputExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult,
    CursorCapture, HudAnchor, HudKind, HudStyle, HudWidget, InputAction, InputKey, NetLocalClient,
    PanelAnchor, PanelDirection, RemoveNetEntity, RetiredEntity, UiButtonEntry, UiGridItem,
    UiGridShape, UiGridSlot, UiGridView, UiGrids, UiHud, UiPanel, UiPanels, UiTooltipLine,
};
use retreat_gameplay_api::{
    ContainerKind, ContainerLayout, EquipSlot, GridItemCatalog, ItemContainer, ItemNotice,
    PlacedItem, SoloPlayer, Wallet, add_gameplay_contract, character_level, container_items,
    local_player_id, playing_body,
};

/// Numbers of the container UI (placeholders; insert before adding the plugin to change).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ContainerUiConfig {
    /// How long an item notice stays on screen.
    pub notice_seconds: f32,
    /// The base whose stacks the bag's footer counts as carried gold.
    pub gold_base: String,
}

impl Default for ContainerUiConfig {
    fn default() -> Self {
        Self {
            notice_seconds: 4.0,
            gold_base: "gold".to_string(),
        }
    }
}

/// Whether this machine shows its containers (UI state of this machine, not simulation).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ContainerWindows {
    pub open: bool,
}

const PANEL: &str = "container_ui";
const PANEL_ENTRIES: &[UiButtonEntry] = &[UiButtonEntry {
    id: "toggle",
    label: "Inventory (I)",
    command_line: "container_ui.toggle",
}];
const NOTICE_WIDGET: &str = "item_notice";
const REQUIREMENT_MET: Color = Color::srgb(0.6, 0.6, 0.65);
const REQUIREMENT_MISSED: Color = Color::srgb(0.95, 0.3, 0.3);
const AFFIX_COLOR: Color = Color::srgb(0.55, 0.75, 1.0);
const DETAIL_COLOR: Color = Color::srgb(0.85, 0.85, 0.85);

/// Adds the container windows, the `container_ui.*` commands and the `I` key.
pub struct ContainerUiModulePlugin;

impl Plugin for ContainerUiModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<ContainerUiConfig>()
            .init_resource::<ContainerWindows>()
            .register_command("container_ui.toggle", "container_ui.toggle", toggle_command)
            .register_command("container_ui.open", "container_ui.open", open_command)
            .register_command("container_ui.close", "container_ui.close", close_command)
            .register_command(
                "assert.items_visible",
                "assert.items_visible <count>",
                assert_visible_command,
            )
            .register_command(
                "assert.item_notice",
                "assert.item_notice <text...>",
                assert_notice_command,
            )
            .register_action(InputAction {
                id: "container_ui_toggle",
                command_line: "container_ui.toggle",
                default_keys: &[InputKey::I],
            })
            .register_panel(UiPanel {
                id: PANEL,
                title: None,
                anchor: PanelAnchor::TopRight,
                direction: PanelDirection::Column,
                entries: PANEL_ENTRIES,
                visible: false,
            })
            .register_hud_widget(HudWidget {
                id: NOTICE_WIDGET,
                anchor: HudAnchor::BottomCenter,
                kind: HudKind::Text { font_size: 22.0 },
                visible: true,
            })
            .add_systems(Update, (show_containers, show_notice).chain());
    }
}

fn set_open(world: &mut World, open: bool) {
    if world.resource::<ContainerWindows>().open == open {
        return;
    }
    world.resource_mut::<ContainerWindows>().open = open;
    if open && let Some(mut cursor) = world.get_resource_mut::<CursorCapture>() {
        // The pointer is needed to drag items.
        cursor.wanted = false;
    }
}

fn toggle_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let open = world.resource::<ContainerWindows>().open;
    set_open(world, !open);
    Ok(CommandOutcome::Continue)
}

fn open_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    set_open(world, true);
    Ok(CommandOutcome::Continue)
}

fn close_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    set_open(world, false);
    Ok(CommandOutcome::Continue)
}

/// The window id of a container kind.
pub fn window_id(kind: ContainerKind) -> &'static str {
    kind.word()
}

/// Where each equipment slot is drawn (in cells of a 6 x 7 canvas), in `EquipSlot::ALL` order.
pub fn equipment_slots() -> Vec<UiGridSlot> {
    EquipSlot::ALL
        .iter()
        .map(|slot| {
            let (x, y, w, h) = match slot {
                EquipSlot::Head => (2, 0, 2, 2),
                EquipSlot::Chest => (2, 2, 2, 3),
                EquipSlot::Hands => (0, 4, 2, 2),
                EquipSlot::Legs => (2, 5, 2, 2),
                EquipSlot::Feet => (4, 4, 2, 2),
                EquipSlot::MainHand => (0, 1, 2, 3),
                EquipSlot::OffHand => (4, 1, 2, 3),
            };
            UiGridSlot {
                label: slot.label().to_string(),
                x,
                y,
                size: UVec2::new(w, h),
            }
        })
        .collect()
}

fn srgb(color: [f32; 3]) -> Color {
    Color::srgb(color[0], color[1], color[2])
}

/// The tooltip of an item for a character of `level`.
pub fn item_tooltip(
    catalog: &GridItemCatalog,
    placed: &PlacedItem,
    level: u32,
) -> Vec<UiTooltipLine> {
    let item = &placed.item;
    let base = catalog.base(&item.base);
    let rarity = catalog.rarity(&item.rarity);
    let name = base.map_or(item.base.as_str(), |base| base.name.as_str());
    let name_color = rarity.map_or(Color::WHITE, |rarity| srgb(rarity.color));
    let mut lines = vec![UiTooltipLine::new(name, name_color)];
    let kind = match base {
        Some(base) if !base.slots.is_empty() => base
            .slots
            .iter()
            .map(|slot| slot.label())
            .collect::<Vec<_>>()
            .join(" / "),
        _ => "Item".to_string(),
    };
    let rarity_name = rarity.map_or(item.rarity.as_str(), |rarity| rarity.name.as_str());
    let stacks = base.is_some_and(|base| base.max_stack > 1);
    if !stacks {
        lines.push(UiTooltipLine::new(
            format!("{rarity_name} - {kind}"),
            DETAIL_COLOR,
        ));
    }
    for rolled in &item.affixes {
        let text = catalog
            .affix(&rolled.affix)
            .map_or(format!("{} {}", rolled.value, rolled.affix), |affix| {
                affix.describe(rolled.value)
            });
        lines.push(UiTooltipLine::new(text, AFFIX_COLOR));
    }
    if let Some(base) = base {
        if stacks {
            lines.push(UiTooltipLine::new(
                format!("{} of {}", item.quantity, base.max_stack),
                DETAIL_COLOR,
            ));
        }
        if base.two_handed {
            lines.push(UiTooltipLine::new("Two-handed", DETAIL_COLOR));
        }
    }
    if item.level_requirement > 1 {
        let color = if level >= item.level_requirement {
            REQUIREMENT_MET
        } else {
            REQUIREMENT_MISSED
        };
        lines.push(UiTooltipLine::new(
            format!("Requires level {}", item.level_requirement),
            color,
        ));
    }
    lines
}

/// The window of `container` holding `items`, for a character of `level` with `balance` in the
/// wallet (none: no wallet). Pure.
pub fn container_view(
    catalog: &GridItemCatalog,
    config: &ContainerUiConfig,
    container: &ItemContainer,
    items: &[PlacedItem],
    level: u32,
    balance: Option<u64>,
) -> UiGridView {
    let (columns, rows, visible_rows, shape) = match &container.layout {
        ContainerLayout::Grid {
            columns,
            rows,
            visible_rows,
        } => (*columns, *rows, *visible_rows, UiGridShape::Cells),
        ContainerLayout::Equipment => (6, 7, 0, UiGridShape::Slots(equipment_slots())),
    };
    let grid_items = items
        .iter()
        .map(|placed| {
            let base = catalog.base(&placed.item.base);
            let base_color = base.map_or([0.4; 3], |base| base.color);
            let rarity_color = catalog
                .rarity(&placed.item.rarity)
                .map_or(base_color, |rarity| rarity.color);
            let tint = |i: usize| base_color[i] * 0.75 + rarity_color[i] * 0.25;
            let label = if placed.item.quantity > 1 {
                placed.item.quantity.to_string()
            } else if base.and_then(|base| base.icon.as_ref()).is_some() {
                String::new()
            } else {
                base.map_or(placed.item.base.clone(), |base| base.name.clone())
            };
            UiGridItem {
                id: placed.item.uid.0.to_string(),
                x: placed.place.x,
                y: placed.place.y,
                size: placed.item.size,
                rotated: placed.place.rotated,
                color: Color::srgb(tint(0), tint(1), tint(2)),
                image: base.and_then(|base| base.icon.clone()),
                label,
                tooltip: item_tooltip(catalog, placed, level),
            }
        })
        .collect();
    let footer = match container.kind {
        ContainerKind::Bag => {
            let carried: u64 = items
                .iter()
                .filter(|placed| placed.item.base == config.gold_base)
                .map(|placed| u64::from(placed.item.quantity))
                .sum();
            match balance {
                Some(balance) => format!("Gold carried: {carried}   Safe balance: {balance}"),
                None => format!("Gold carried: {carried}"),
            }
        }
        ContainerKind::Equipment => format!("Level {level}"),
        ContainerKind::Chest => "Safe: never lost".to_string(),
    };
    let order = match container.kind {
        ContainerKind::Equipment => 0,
        ContainerKind::Bag => 1,
        ContainerKind::Chest => 2,
    };
    UiGridView {
        title: container.title.clone(),
        columns,
        rows,
        visible_rows,
        shape,
        items: grid_items,
        drop_command: container.drop_command.clone(),
        order,
        footer,
    }
}

/// Writes this machine's containers into the engine's grid windows (only what changed).
fn show_containers(world: &mut World, mut chest_was_open: Local<bool>) {
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    );
    let playing = player.and_then(|player| playing_body(world, player));
    let panel_visible = playing.is_some();
    if world.resource::<UiPanels>().is_visible(PANEL) != panel_visible {
        world
            .resource_mut::<UiPanels>()
            .set_visible(PANEL, panel_visible)
            .expect("registered");
    }
    let mut wanted: Vec<(&'static str, UiGridView)> = Vec::new();
    if let Some((body, character)) = playing {
        let containers: Vec<ItemContainer> = world
            .query_filtered::<&ItemContainer, (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
            .iter(world)
            .filter(|container| container.character == character.id)
            .cloned()
            .collect();
        let chest_open = containers.iter().any(|c| c.kind == ContainerKind::Chest);
        if chest_open && !*chest_was_open {
            set_open(world, true);
        }
        *chest_was_open = chest_open;
        if world.resource::<ContainerWindows>().open {
            let level = character_level(world, body);
            let balance = world
                .query_filtered::<&Wallet, (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
                .iter(world)
                .find(|wallet| wallet.character == character.id)
                .map(|wallet| wallet.balance);
            let catalog = world.resource::<GridItemCatalog>().clone();
            let config = world.resource::<ContainerUiConfig>().clone();
            for container in containers {
                let items = container_items(world, &container.id);
                let view = container_view(&catalog, &config, &container, &items, level, balance);
                wanted.push((window_id(container.kind), view));
            }
        }
    } else {
        *chest_was_open = false;
    }
    let shown: Vec<String> = world
        .resource::<UiGrids>()
        .views()
        .map(|(id, _)| id.to_string())
        .collect();
    for kind in [
        ContainerKind::Bag,
        ContainerKind::Equipment,
        ContainerKind::Chest,
    ] {
        let id = window_id(kind);
        match wanted.iter().find(|(wanted_id, _)| *wanted_id == id) {
            Some((_, view)) => {
                if world.resource::<UiGrids>().view(id) != Some(view) {
                    world.resource_mut::<UiGrids>().set_view(id, view.clone());
                }
            }
            None => {
                if shown.iter().any(|shown_id| shown_id == id) {
                    world.resource_mut::<UiGrids>().remove_view(id);
                }
            }
        }
    }
}

/// Shows a new item notice for a few seconds.
fn show_notice(world: &mut World, mut last: Local<Option<(u32, f32)>>) {
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    );
    let notice = player
        .and_then(|player| playing_body(world, player))
        .and_then(|(body, _)| world.get::<ItemNotice>(body).cloned());
    let seconds = world.resource::<ContainerUiConfig>().notice_seconds;
    let mut text = String::new();
    if let Some(notice) = notice {
        if last.is_none_or(|(serial, _)| serial != notice.serial) {
            *last = Some((notice.serial, now));
        }
        if last.is_some_and(|(_, since)| now - since < seconds) {
            text = notice.text;
        }
    }
    let mut hud = world.resource_mut::<UiHud>();
    if !hud.has_text(NOTICE_WIDGET, &text) {
        hud.set_text(NOTICE_WIDGET, text).expect("registered");
        hud.set_style(
            NOTICE_WIDGET,
            HudStyle {
                color: Color::srgb(1.0, 0.85, 0.4),
                panel: Some(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            },
        )
        .expect("registered");
    }
}

/// `assert.item_notice <text...>`: the last item notice of this machine's character is this text
/// (a refused move, a full bag; refusals are notices, not errors).
fn assert_notice_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let expected = args.rest(0).join(" ");
    if expected.is_empty() {
        return Err(args.failure("expected the notice text"));
    }
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
    .ok_or_else(|| args.failure("this machine has no player"))?;
    let (body, _) = playing_body(world, player)
        .ok_or_else(|| args.failure("this machine's player is not playing a character"))?;
    let actual = world
        .get::<ItemNotice>(body)
        .map_or(String::new(), |notice| notice.text.clone());
    if actual == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "the item notice is `{actual}`, expected `{expected}`"
        )))
    }
}

/// `assert.items_visible <count>`: how many items this machine knows of (a client knows only its
/// own player's items: they are private).
fn assert_visible_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "count")?;
    let actual = world
        .query_filtered::<&retreat_gameplay_api::GridItem, (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .count();
    if actual == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "this machine knows {actual} items, expected {expected}"
        )))
    }
}
