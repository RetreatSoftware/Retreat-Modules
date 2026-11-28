//! Item containers (crawler plan, phase 2): a character's bag, equipment, town chest (later corpse,
//! vendor, trade offer). A container is an entity with [`ItemContainer`]; every item in it is an
//! entity with [`crate::GridItem`] and an [`ItemPlace`] naming the container. Both are shared state,
//! private to the owning player (`NetPrivateTo`).
//!
//! **Who owns what.** The module that makes a container owns it: it spawns it (and its items) when
//! the character enters, loads and keeps its record, removes it when the character leaves, and
//! registers its drop command (`ItemContainer::drop_command`, run by the container UI as
//! `<drop_command> <uid> <x> <y> <0|1>`). A move between containers of different modules goes through
//! the one contract operation [`crate::move_item`], which checks both sides' rules from the data here
//! and writes both containers' records in one atomic batch.
//!
//! **Pure rules** (tested here): [`fits_in_grid`], [`first_free_spot`], [`check_equip`],
//! [`plan_move`].

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

use crate::{CharacterId, EquipSlot, GridItem, GridItemCatalog, ItemUid};

/// A container, unique on the server: `<kind>/<character>` (`bag/alice-1`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContainerId(pub String);

/// What a container is (decides how the UI shows it and which rules apply).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerKind {
    Bag,
    Equipment,
    Chest,
}

impl ContainerKind {
    /// The word used in ids and entity names.
    pub fn word(self) -> &'static str {
        match self {
            ContainerKind::Bag => "bag",
            ContainerKind::Equipment => "equipment",
            ContainerKind::Chest => "chest",
        }
    }
}

/// The id of a character's container of `kind`.
pub fn container_id(kind: ContainerKind, character: &CharacterId) -> ContainerId {
    ContainerId(format!("{}/{}", kind.word(), character.0))
}

/// The entity name of a container: `container_<kind>_<character>`.
pub fn container_entity_name(id: &ContainerId) -> String {
    format!("container_{}", id.0.replace('/', "_"))
}

/// How a container holds items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerLayout {
    /// A grid: items take `size` cells from their top-left cell. `visible_rows` is how many rows a
    /// window shows at once (0: all; fewer scroll).
    Grid {
        columns: u32,
        rows: u32,
        visible_rows: u32,
    },
    /// One item per [`EquipSlot`]; an item's place `x` is the slot index.
    Equipment,
}

/// A container (on its entity; shared, private to the owning player).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemContainer {
    pub id: ContainerId,
    pub kind: ContainerKind,
    /// Window title.
    pub title: String,
    pub layout: ContainerLayout,
    /// The character whose container it is: only that character's player may move its items.
    pub character: CharacterId,
    /// Items may go in or out only while the character stands in a safe area (the town chest).
    pub only_in_safe_area: bool,
    /// The command a drop on it runs: `<drop_command> <uid> <x> <y> <0|1>`.
    pub drop_command: String,
    /// Where its items are saved: record owner and part (the owning module's id).
    pub record_owner: String,
    pub record_part: String,
}

impl NetState for ItemContainer {
    const ID: &'static str = "gameplay.item_container";
}

/// Where an item is (on the item entity; shared, private).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemPlace {
    pub container: ContainerId,
    /// Top-left cell of a grid, or the slot index of equipment.
    pub x: u32,
    pub y: u32,
    /// Turned 90 degrees in a grid (covers `size.y` x `size.x` cells).
    pub rotated: bool,
}

impl NetState for ItemPlace {
    const ID: &'static str = "gameplay.item_place";
}

/// The cells an item covers as placed.
pub fn placed_size(size: UVec2, rotated: bool) -> UVec2 {
    if rotated {
        UVec2::new(size.y, size.x)
    } else {
        size
    }
}

/// An item and where it is, as rules see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedItem {
    pub item: GridItem,
    pub place: ItemPlace,
}

fn overlaps(a: UVec2, a_size: UVec2, b: UVec2, b_size: UVec2) -> bool {
    a.x < b.x + b_size.x && b.x < a.x + a_size.x && a.y < b.y + b_size.y && b.y < a.y + a_size.y
}

/// The items of `items` (except `ignore`) that the cells `at`..`at + size` touch.
pub fn items_under(
    items: &[PlacedItem],
    ignore: ItemUid,
    at: UVec2,
    size: UVec2,
) -> Vec<&PlacedItem> {
    items
        .iter()
        .filter(|other| other.item.uid != ignore)
        .filter(|other| {
            let other_at = UVec2::new(other.place.x, other.place.y);
            overlaps(
                at,
                size,
                other_at,
                placed_size(other.item.size, other.place.rotated),
            )
        })
        .collect()
}

/// Whether cells `at`..`at + size` lie inside a `columns` x `rows` grid and touch no item of
/// `items` other than `ignore`.
pub fn fits_in_grid(
    columns: u32,
    rows: u32,
    items: &[PlacedItem],
    ignore: ItemUid,
    at: UVec2,
    size: UVec2,
) -> bool {
    at.x + size.x <= columns
        && at.y + size.y <= rows
        && items_under(items, ignore, at, size).is_empty()
}

/// The first free place for an item of `size` in a grid: row by row, left to right, upright first
/// and then turned. `None` when it fits nowhere.
pub fn first_free_spot(
    columns: u32,
    rows: u32,
    items: &[PlacedItem],
    size: UVec2,
) -> Option<(UVec2, bool)> {
    for rotated in [false, true] {
        let size = placed_size(size, rotated);
        for y in 0..rows {
            for x in 0..columns {
                let at = UVec2::new(x, y);
                // Nothing is ignored: every listed item takes its cells.
                let free = at.x + size.x <= columns
                    && at.y + size.y <= rows
                    && items.iter().all(|other| {
                        let other_at = UVec2::new(other.place.x, other.place.y);
                        !overlaps(
                            at,
                            size,
                            other_at,
                            placed_size(other.item.size, other.place.rotated),
                        )
                    });
                if free {
                    return Some((at, rotated));
                }
            }
        }
        if size.x == size.y {
            break;
        }
    }
    None
}

/// Whether `item` may go in `slot` of an equipment container holding `equipped` (by a character of
/// `level`); the error is for the player. Rules: the base lists the slot, the level is reached, the
/// slot is empty, a two-handed item needs the off hand empty, nothing goes in the off hand while
/// the main hand holds a two-handed item.
pub fn check_equip(
    catalog: &GridItemCatalog,
    item: &GridItem,
    slot: EquipSlot,
    equipped: &[PlacedItem],
    level: u32,
) -> Result<(), String> {
    let base = catalog
        .base(&item.base)
        .ok_or_else(|| format!("unknown item `{}`", item.base))?;
    if !base.slots.contains(&slot) {
        return Err(format!(
            "{} cannot be worn in the {} slot",
            base.name,
            slot.label().to_lowercase()
        ));
    }
    if level < item.level_requirement {
        return Err(format!(
            "{} needs level {} (you are level {level})",
            base.name, item.level_requirement
        ));
    }
    let in_slot = |wanted: EquipSlot| {
        equipped
            .iter()
            .find(|other| other.item.uid != item.uid && other.place.x == wanted.index())
    };
    if let Some(other) = in_slot(slot) {
        let name = catalog
            .base(&other.item.base)
            .map_or("an item", |b| b.name.as_str());
        return Err(format!(
            "the {} slot holds {name}; take it off first",
            slot.label().to_lowercase()
        ));
    }
    if base.two_handed && in_slot(EquipSlot::OffHand).is_some() {
        return Err(format!(
            "{} needs both hands; empty the off hand first",
            base.name
        ));
    }
    if slot == EquipSlot::OffHand
        && let Some(main) = in_slot(EquipSlot::MainHand)
        && catalog.base(&main.item.base).is_some_and(|b| b.two_handed)
    {
        return Err("the main hand holds a two-handed item".to_string());
    }
    Ok(())
}

/// What a move does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveOutcome {
    /// The item goes to this place.
    Placed(ItemPlace),
    /// `amount` of the item joins the stack `into`; the rest (if any) stays where it was.
    Stacked { into: ItemUid, amount: u32 },
}

/// The facts a move is judged on.
#[derive(Debug, Clone)]
pub struct MoveContext<'a> {
    pub catalog: &'a GridItemCatalog,
    /// The level of the moving character (1 without progression).
    pub level: u32,
    /// Whether the character stands in a safe area.
    pub in_safe_area: bool,
}

/// Decides moving `item` (now in `from`) to `(x, y, rotated)` of `to`, which holds `to_items`. Pure;
/// the error is for the player. Ownership is checked by the caller.
#[allow(clippy::too_many_arguments)] // The move's facts, read plainly at the call.
pub fn plan_move(
    context: &MoveContext,
    item: &GridItem,
    from: &ItemContainer,
    to: &ItemContainer,
    to_items: &[PlacedItem],
    x: u32,
    y: u32,
    rotated: bool,
) -> Result<MoveOutcome, String> {
    for container in [from, to] {
        if container.only_in_safe_area && !context.in_safe_area {
            return Err(format!(
                "the {} can only be used in the city",
                container.title.to_lowercase()
            ));
        }
    }
    match &to.layout {
        ContainerLayout::Equipment => {
            let slot = EquipSlot::from_index(x).ok_or_else(|| format!("there is no slot {x}"))?;
            check_equip(context.catalog, item, slot, to_items, context.level)?;
            Ok(MoveOutcome::Placed(ItemPlace {
                container: to.id.clone(),
                x,
                y: 0,
                rotated: false,
            }))
        }
        ContainerLayout::Grid { columns, rows, .. } => {
            let at = UVec2::new(x, y);
            let size = placed_size(item.size, rotated);
            let under = items_under(to_items, item.uid, at, size);
            let max_stack = context
                .catalog
                .base(&item.base)
                .map_or(1, |base| base.max_stack);
            if let [only] = under.as_slice()
                && only.item.base == item.base
                && max_stack > 1
            {
                let room = max_stack.saturating_sub(only.item.quantity);
                if room == 0 {
                    return Err("that stack is full".to_string());
                }
                return Ok(MoveOutcome::Stacked {
                    into: only.item.uid,
                    amount: room.min(item.quantity),
                });
            }
            if !fits_in_grid(*columns, *rows, to_items, item.uid, at, size) {
                return Err(format!(
                    "it does not fit there in the {}",
                    to.title.to_lowercase()
                ));
            }
            Ok(MoveOutcome::Placed(ItemPlace {
                container: to.id.clone(),
                x,
                y,
                rotated,
            }))
        }
    }
}

/// A new stack [`plan_stack_add`] wants: its top-left cell, whether it is turned, how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewStack {
    pub at: UVec2,
    pub rotated: bool,
    pub quantity: u32,
}

/// Where `quantity` of the stackable `base` (upright `size`, at most `max_stack` per stack) goes in
/// a `columns` x `rows` grid holding `items`: first onto stacks of the same base (by uid), then into
/// new stacks at free spots. Returns the grown stacks and the new stacks, or `None` when not all of
/// it fits. Pure.
pub fn plan_stack_add(
    columns: u32,
    rows: u32,
    items: &[PlacedItem],
    base: &str,
    size: UVec2,
    max_stack: u32,
    quantity: u32,
) -> Option<(Vec<PlacedItem>, Vec<NewStack>)> {
    let max_stack = max_stack.max(1);
    let mut left = quantity;
    let mut grown = Vec::new();
    let mut stacks: Vec<&PlacedItem> = items.iter().filter(|p| p.item.base == base).collect();
    stacks.sort_by_key(|p| p.item.uid);
    for stack in stacks {
        if left == 0 {
            break;
        }
        let room = max_stack.saturating_sub(stack.item.quantity).min(left);
        if room > 0 {
            let mut more = stack.clone();
            more.item.quantity += room;
            left -= room;
            grown.push(more);
        }
    }
    let mut taken: Vec<PlacedItem> = items.to_vec();
    let mut new_stacks = Vec::new();
    while left > 0 {
        let (at, rotated) = first_free_spot(columns, rows, &taken, size)?;
        let amount = left.min(max_stack);
        left -= amount;
        new_stacks.push(NewStack {
            at,
            rotated,
            quantity: amount,
        });
        // Occupy the spot for the next stack (a uid no real item has).
        let mut placeholder = items.first().cloned().unwrap_or_else(|| PlacedItem {
            item: GridItem {
                uid: ItemUid(0),
                base: String::new(),
                rarity: String::new(),
                affixes: Vec::new(),
                level_requirement: 0,
                quantity: 0,
                size,
            },
            place: ItemPlace {
                container: ContainerId(String::new()),
                x: 0,
                y: 0,
                rotated: false,
            },
        });
        placeholder.item.uid = ItemUid(u64::MAX - new_stacks.len() as u64);
        placeholder.item.size = size;
        placeholder.place.x = at.x;
        placeholder.place.y = at.y;
        placeholder.place.rotated = rotated;
        taken.push(placeholder);
    }
    Some((grown, new_stacks))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ItemBase, ItemRarity};

    fn base(
        id: &str,
        size: UVec2,
        slots: Vec<EquipSlot>,
        two_handed: bool,
        max_stack: u32,
    ) -> ItemBase {
        ItemBase {
            id: id.to_string(),
            name: id.to_string(),
            size,
            slots,
            two_handed,
            max_stack,
            rollable: true,
            affix_pool: Vec::new(),
            icon: None,
            color: [0.5; 3],
        }
    }

    fn catalog() -> GridItemCatalog {
        GridItemCatalog {
            bases: vec![
                base(
                    "sword",
                    UVec2::new(1, 3),
                    vec![EquipSlot::MainHand],
                    false,
                    1,
                ),
                base(
                    "greatsword",
                    UVec2::new(2, 4),
                    vec![EquipSlot::MainHand],
                    true,
                    1,
                ),
                base(
                    "shield",
                    UVec2::new(2, 2),
                    vec![EquipSlot::OffHand],
                    false,
                    1,
                ),
                base("gold", UVec2::ONE, Vec::new(), false, 100),
            ],
            rarities: vec![ItemRarity {
                id: "common".to_string(),
                name: "Common".to_string(),
                weight: 1,
                affixes: (0, 0),
                color: [1.0; 3],
            }],
            affixes: Vec::new(),
        }
    }

    fn item(uid: u64, base: &str, quantity: u32, level: u32) -> GridItem {
        let size = catalog().base(base).unwrap().size;
        GridItem {
            uid: ItemUid(uid),
            base: base.to_string(),
            rarity: "common".to_string(),
            affixes: Vec::new(),
            level_requirement: level,
            quantity,
            size,
        }
    }

    fn container(kind: ContainerKind, layout: ContainerLayout, safe_only: bool) -> ItemContainer {
        let character = CharacterId("ada-1".to_string());
        ItemContainer {
            id: container_id(kind, &character),
            kind,
            title: kind.word().to_string(),
            layout,
            character,
            only_in_safe_area: safe_only,
            drop_command: format!("{}.put", kind.word()),
            record_owner: "character/ada-1".to_string(),
            record_part: kind.word().to_string(),
        }
    }

    fn bag() -> ItemContainer {
        container(
            ContainerKind::Bag,
            ContainerLayout::Grid {
                columns: 4,
                rows: 4,
                visible_rows: 0,
            },
            false,
        )
    }

    fn placed(
        item: GridItem,
        container: &ItemContainer,
        x: u32,
        y: u32,
        rotated: bool,
    ) -> PlacedItem {
        PlacedItem {
            item,
            place: ItemPlace {
                container: container.id.clone(),
                x,
                y,
                rotated,
            },
        }
    }

    fn context(catalog: &GridItemCatalog, level: u32, safe: bool) -> MoveContext<'_> {
        MoveContext {
            catalog,
            level,
            in_safe_area: safe,
        }
    }

    #[test]
    fn free_spots_go_row_by_row_and_turn_an_item_that_fits_only_turned() {
        let bag = bag();
        let items = vec![placed(item(1, "shield", 1, 1), &bag, 0, 0, false)];
        assert_eq!(
            first_free_spot(4, 4, &items, UVec2::new(2, 2)),
            Some((UVec2::new(2, 0), false))
        );
        // A 4x1 item in a 3-wide, 4-high grid fits only standing.
        assert_eq!(
            first_free_spot(3, 4, &[], UVec2::new(4, 1)),
            Some((UVec2::new(0, 0), true))
        );
        let full: Vec<PlacedItem> = (0..4)
            .map(|i| placed(item(10 + i, "sword", 1, 1), &bag, i as u32, 0, false))
            .collect();
        assert_eq!(first_free_spot(4, 3, &full, UVec2::new(1, 1)), None);
    }

    #[test]
    fn a_grid_move_needs_free_cells_and_stacks_onto_the_same_base() {
        let catalog = catalog();
        let bag = bag();
        let items = vec![
            placed(item(1, "shield", 1, 1), &bag, 0, 0, false),
            placed(item(2, "gold", 90, 1), &bag, 3, 3, false),
        ];
        let sword = item(3, "sword", 1, 1);
        let ctx = context(&catalog, 1, false);
        assert!(plan_move(&ctx, &sword, &bag, &bag, &items, 1, 0, false).is_err());
        assert_eq!(
            plan_move(&ctx, &sword, &bag, &bag, &items, 0, 2, true),
            Ok(MoveOutcome::Placed(ItemPlace {
                container: bag.id.clone(),
                x: 0,
                y: 2,
                rotated: true
            }))
        );
        let gold = item(4, "gold", 25, 1);
        assert_eq!(
            plan_move(&ctx, &gold, &bag, &bag, &items, 3, 3, false),
            Ok(MoveOutcome::Stacked {
                into: ItemUid(2),
                amount: 10
            })
        );
    }

    #[test]
    fn equipping_checks_slot_level_and_hands() {
        let catalog = catalog();
        let equipment = container(ContainerKind::Equipment, ContainerLayout::Equipment, false);
        let main = EquipSlot::MainHand.index();
        let off = EquipSlot::OffHand.index();
        let ctx = context(&catalog, 3, false);
        let sword = item(1, "sword", 1, 5);
        let refusal = plan_move(&ctx, &sword, &bag(), &equipment, &[], main, 0, false).unwrap_err();
        assert!(
            refusal.contains("needs level 5 (you are level 3)"),
            "{refusal}"
        );
        let shield = item(2, "shield", 1, 1);
        assert!(
            plan_move(&ctx, &shield, &bag(), &equipment, &[], main, 0, false)
                .unwrap_err()
                .contains("cannot be worn")
        );
        let with_shield = vec![placed(shield.clone(), &equipment, off, 0, false)];
        let great = item(3, "greatsword", 1, 1);
        assert!(
            plan_move(
                &ctx,
                &great,
                &bag(),
                &equipment,
                &with_shield,
                main,
                0,
                false
            )
            .unwrap_err()
            .contains("both hands")
        );
        let with_great = vec![placed(great, &equipment, main, 0, false)];
        assert!(
            plan_move(
                &ctx,
                &shield,
                &bag(),
                &equipment,
                &with_great,
                off,
                0,
                false
            )
            .unwrap_err()
            .contains("two-handed")
        );
        assert!(
            plan_move(
                &ctx,
                &item(4, "sword", 1, 1),
                &bag(),
                &equipment,
                &with_great,
                main,
                0,
                false
            )
            .unwrap_err()
            .contains("take it off first")
        );
    }

    #[test]
    fn a_city_only_container_refuses_outside_a_safe_area() {
        let catalog = catalog();
        let chest = container(
            ContainerKind::Chest,
            ContainerLayout::Grid {
                columns: 8,
                rows: 8,
                visible_rows: 4,
            },
            true,
        );
        let sword = item(1, "sword", 1, 1);
        assert!(
            plan_move(
                &context(&catalog, 1, false),
                &sword,
                &bag(),
                &chest,
                &[],
                0,
                0,
                false
            )
            .unwrap_err()
            .contains("only be used in the city")
        );
        assert!(
            plan_move(
                &context(&catalog, 1, true),
                &sword,
                &bag(),
                &chest,
                &[],
                0,
                0,
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn gold_fills_existing_stacks_first_then_new_ones_or_refuses_when_it_cannot_fit() {
        let bag = bag();
        let items = vec![
            placed(item(1, "gold", 70, 1), &bag, 0, 0, false),
            placed(item(2, "shield", 1, 1), &bag, 1, 0, false),
        ];
        let (grown, new) = plan_stack_add(4, 4, &items, "gold", UVec2::ONE, 100, 250).unwrap();
        assert_eq!(grown.len(), 1);
        assert_eq!(grown[0].item.quantity, 100);
        assert_eq!(
            new,
            vec![
                NewStack {
                    at: UVec2::new(3, 0),
                    rotated: false,
                    quantity: 100
                },
                NewStack {
                    at: UVec2::new(0, 1),
                    rotated: false,
                    quantity: 100
                },
                NewStack {
                    at: UVec2::new(3, 1),
                    rotated: false,
                    quantity: 20
                },
            ]
        );
        assert!(plan_stack_add(1, 1, &[], "gold", UVec2::ONE, 100, 101).is_none());
    }

    #[test]
    fn ids_and_names_are_derived_from_kind_and_character() {
        let id = container_id(ContainerKind::Chest, &CharacterId("bob-2".to_string()));
        assert_eq!(id.0, "chest/bob-2");
        assert_eq!(container_entity_name(&id), "container_chest_bob-2");
    }
}
