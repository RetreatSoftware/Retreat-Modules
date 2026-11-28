//! Grid items (crawler plan, phase 2): item instances with a stable unique id, rolled from game data,
//! that take `W x H` cells in containers. The stack `Inventory` of the arena (`item.rs`) is a
//! different, older model; a game uses one or the other.
//!
//! **Data** ([`GridItemCatalog`], the game's resource): item bases (name, size, equipment slots, stack
//! size, look), rarity tiers and affixes. The `item_rolls` module rolls instances from it; every
//! other module reads it to know what a base is.
//!
//! **Instances** ([`GridItem`]): one entity per item, named [`item_entity_name`] (`item_<uid>`), with
//! [`GridItem`] and its [`crate::ItemPlace`] (shared state, private to the owning player). The uid is
//! unique on the server for ever ([`crate::allocate_item_uid`]), so an item can be followed through
//! every move and a duplicate can be detected. The size is copied from the base when the item is
//! made, so a saved placement stays valid when the base data changes.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// The unique id of an item instance on a server; never reused.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct ItemUid(pub u64);

/// The entity name of an item: `item_<uid>`.
pub fn item_entity_name(uid: ItemUid) -> String {
    format!("item_{}", uid.0)
}

/// An equipment slot (decided with the user, 2026-11-27: no rings, amulet or back slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Head,
    Chest,
    Hands,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

impl EquipSlot {
    /// Every slot, in the order of their index in an equipment container.
    pub const ALL: [EquipSlot; 7] = [
        EquipSlot::Head,
        EquipSlot::Chest,
        EquipSlot::Hands,
        EquipSlot::Legs,
        EquipSlot::Feet,
        EquipSlot::MainHand,
        EquipSlot::OffHand,
    ];

    /// The slot's index in an equipment container (its place `x`).
    pub fn index(self) -> u32 {
        Self::ALL
            .iter()
            .position(|slot| *slot == self)
            .expect("listed") as u32
    }

    pub fn from_index(index: u32) -> Option<EquipSlot> {
        Self::ALL.get(index as usize).copied()
    }

    /// The name players see.
    pub fn label(self) -> &'static str {
        match self {
            EquipSlot::Head => "Head",
            EquipSlot::Chest => "Chest",
            EquipSlot::Hands => "Hands",
            EquipSlot::Legs => "Legs",
            EquipSlot::Feet => "Feet",
            EquipSlot::MainHand => "Main hand",
            EquipSlot::OffHand => "Off hand",
        }
    }
}

/// A kind of item (game data).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemBase {
    /// Stable id, one word (`short_sword`); saved in records.
    pub id: String,
    pub name: String,
    /// Cells it takes, upright.
    pub size: UVec2,
    /// The slots it can be equipped in (none: not equipment).
    pub slots: Vec<EquipSlot>,
    /// Takes the main hand and needs the off hand empty.
    pub two_handed: bool,
    /// Most items in one stack (1: never stacks).
    pub max_stack: u32,
    /// Whether random rolls may make it (`false` for gold and other fixed things).
    pub rollable: bool,
    /// The affixes it may roll (ids from [`GridItemCatalog::affixes`]).
    pub affix_pool: Vec<String>,
    /// Icon (PNG in the game's assets), or none.
    pub icon: Option<String>,
    /// Background of its cells (sRGB).
    pub color: [f32; 3],
}

/// A rarity tier (game data).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemRarity {
    /// Stable id (`common`); saved in records.
    pub id: String,
    pub name: String,
    /// How often rolls pick it, relative to the others.
    pub weight: u32,
    /// How many affixes an item of this rarity rolls (inclusive).
    pub affixes: (u32, u32),
    /// Color of its name (sRGB).
    pub color: [f32; 3],
}

/// An affix (game data): a rolled number shown with a text.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemAffix {
    /// Stable id (`strength`); saved in records.
    pub id: String,
    /// The text, with `{}` where the number goes (`+{} strength`).
    pub text: String,
    /// The rolled number at item level 1 (inclusive).
    pub min: i32,
    pub max: i32,
    /// Added to both ends per item level above 1.
    pub per_level: i32,
}

impl ItemAffix {
    /// The text of a rolled value.
    pub fn describe(&self, value: i32) -> String {
        self.text.replacen("{}", &value.to_string(), 1)
    }
}

/// All grid item data of a game (a resource; empty until the game inserts its own).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct GridItemCatalog {
    pub bases: Vec<ItemBase>,
    /// From most common to rarest.
    pub rarities: Vec<ItemRarity>,
    pub affixes: Vec<ItemAffix>,
}

fn is_word(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl GridItemCatalog {
    pub fn base(&self, id: &str) -> Option<&ItemBase> {
        self.bases.iter().find(|base| base.id == id)
    }

    pub fn rarity(&self, id: &str) -> Option<&ItemRarity> {
        self.rarities.iter().find(|rarity| rarity.id == id)
    }

    pub fn affix(&self, id: &str) -> Option<&ItemAffix> {
        self.affixes.iter().find(|affix| affix.id == id)
    }

    /// Every problem of the data, in plain language (empty when it is valid).
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut check_ids = |kind: &str, ids: Vec<&str>| {
            for (index, id) in ids.iter().enumerate() {
                if !is_word(id) {
                    problems.push(format!(
                        "{kind} id `{id}` must be lowercase letters, digits and `_`"
                    ));
                }
                if ids[..index].contains(id) {
                    problems.push(format!("{kind} id `{id}` is used twice"));
                }
            }
        };
        check_ids(
            "item base",
            self.bases.iter().map(|b| b.id.as_str()).collect(),
        );
        check_ids(
            "rarity",
            self.rarities.iter().map(|r| r.id.as_str()).collect(),
        );
        check_ids(
            "affix",
            self.affixes.iter().map(|a| a.id.as_str()).collect(),
        );
        for base in &self.bases {
            if base.size.x == 0 || base.size.y == 0 {
                problems.push(format!("item base `{}` has an empty size", base.id));
            }
            if base.max_stack == 0 {
                problems.push(format!("item base `{}` has max_stack 0; use 1", base.id));
            }
            if base.two_handed && !base.slots.contains(&EquipSlot::MainHand) {
                problems.push(format!(
                    "item base `{}` is two-handed but cannot go in the main hand",
                    base.id
                ));
            }
            for affix in &base.affix_pool {
                if self.affix(affix).is_none() {
                    problems.push(format!(
                        "item base `{}` lists unknown affix `{affix}`",
                        base.id
                    ));
                }
            }
        }
        if !self.bases.is_empty() && self.rarities.is_empty() {
            problems.push("there are item bases but no rarity".to_string());
        }
        for rarity in &self.rarities {
            if rarity.affixes.0 > rarity.affixes.1 {
                problems.push(format!(
                    "rarity `{}` rolls at least {} affixes but at most {}",
                    rarity.id, rarity.affixes.0, rarity.affixes.1
                ));
            }
        }
        if !self.rarities.is_empty() && self.rarities.iter().all(|r| r.weight == 0) {
            problems.push("every rarity has weight 0, so none can be rolled".to_string());
        }
        for affix in &self.affixes {
            if affix.min > affix.max {
                problems.push(format!(
                    "affix `{}` has min {} above max {}",
                    affix.id, affix.min, affix.max
                ));
            }
            if !affix.text.contains("{}") {
                problems.push(format!(
                    "affix `{}` text has no `{{}}` for its number",
                    affix.id
                ));
            }
        }
        problems
    }
}

/// One rolled affix of an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolledAffix {
    pub affix: String,
    pub value: i32,
}

/// An item instance (on its entity `item_<uid>`; shared, private to the owner).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridItem {
    pub uid: ItemUid,
    /// [`ItemBase::id`].
    pub base: String,
    /// [`ItemRarity::id`].
    pub rarity: String,
    pub affixes: Vec<RolledAffix>,
    /// The character level needed to equip it.
    pub level_requirement: u32,
    /// How many are in this stack (1 for what does not stack).
    pub quantity: u32,
    /// Cells it takes upright (copied from the base when made).
    pub size: UVec2,
}

impl NetState for GridItem {
    const ID: &'static str = "gameplay.grid_item";
}

/// A character's level (on its body; shared). Owned by the progression module (crawler phase 3);
/// a body without it is level 1.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterLevel(pub u32);

impl NetState for CharacterLevel {
    const ID: &'static str = "gameplay.character_level";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(id: &str) -> ItemBase {
        ItemBase {
            id: id.to_string(),
            name: id.to_string(),
            size: UVec2::new(1, 2),
            slots: vec![EquipSlot::MainHand],
            two_handed: false,
            max_stack: 1,
            rollable: true,
            affix_pool: vec!["strength".to_string()],
            icon: None,
            color: [0.5, 0.5, 0.5],
        }
    }

    fn catalog() -> GridItemCatalog {
        GridItemCatalog {
            bases: vec![base("sword")],
            rarities: vec![ItemRarity {
                id: "common".to_string(),
                name: "Common".to_string(),
                weight: 1,
                affixes: (0, 1),
                color: [1.0, 1.0, 1.0],
            }],
            affixes: vec![ItemAffix {
                id: "strength".to_string(),
                text: "+{} strength".to_string(),
                min: 1,
                max: 3,
                per_level: 1,
            }],
        }
    }

    #[test]
    fn a_valid_catalog_has_no_problems_and_affixes_describe_their_value() {
        assert_eq!(catalog().problems(), Vec::<String>::new());
        assert_eq!(catalog().affixes[0].describe(4), "+4 strength");
    }

    #[test]
    fn problems_name_the_bad_entry() {
        let mut bad = catalog();
        bad.bases.push(base("sword"));
        bad.bases[0].affix_pool.push("luck".to_string());
        bad.bases[0].two_handed = true;
        bad.bases[0].slots.clear();
        let problems = bad.problems().join("\n");
        assert!(problems.contains("`sword` is used twice"), "{problems}");
        assert!(problems.contains("unknown affix `luck`"), "{problems}");
        assert!(
            problems.contains("two-handed but cannot go in the main hand"),
            "{problems}"
        );
    }

    #[test]
    fn slots_have_fixed_indexes() {
        for slot in EquipSlot::ALL {
            assert_eq!(EquipSlot::from_index(slot.index()), Some(slot));
        }
        assert_eq!(EquipSlot::from_index(7), None);
    }
}
