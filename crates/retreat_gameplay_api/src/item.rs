//! Items and inventories: what a player carries, as data.
//!
//! Who writes what (decided 2026-11-26):
//! - the game inserts the [`ItemCatalog`] (what items exist; the same on server and clients);
//! - the inventory module owns [`Inventory`]: it creates it, applies [`GrantRequested`], uses items
//!   and empties it on death ([`ItemsLost`]) and on a new match;
//! - a weapon module only takes its own ammunition out, through [`AmmoSource`];
//! - health acts on [`HealRequested`].
//!
//! Nobody needs the inventory module: without an [`Inventory`] weapons are carried and their reserve
//! is unlimited ([`owns_weapon`], [`AmmoSource`]). A later loot module asks [`Inventory::room_for`]
//! whether a pickup can be taken, grants it with [`GrantRequested`] and drops what [`ItemsLost`]
//! names. Item ids are stable strings, so an inventory can be saved one day.

use crate::{PlayerId, WeaponKind};
use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// Names one kind of item (`health_kit`). One word, so commands can take it; never renamed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl ItemId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for ItemId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// What an item does. Modules act on the kind, never on a particular id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// Reserve ammunition of a weapon, one per round.
    Ammo(WeaponKind),
    /// Carrying the weapon. A weapon with no such item in the catalog is carried by everyone.
    Weapon(WeaponKind),
    /// Used up to restore hit points.
    Heal(u32),
}

/// One item of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinition {
    pub id: ItemId,
    /// What a player reads.
    pub name: String,
    pub kind: ItemKind,
    /// Most of it one player carries.
    pub max_count: u32,
}

impl ItemDefinition {
    pub fn new(id: &str, name: &str, kind: ItemKind, max_count: u32) -> Self {
        Self {
            id: ItemId::new(id),
            name: name.to_string(),
            kind,
            max_count,
        }
    }
}

/// Every item of the game. Empty by default (no items); a game inserts its own before adding any
/// module.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemCatalog(pub Vec<ItemDefinition>);

impl ItemCatalog {
    /// What is wrong, naming the item.
    pub fn validate(&self) -> Result<(), String> {
        for (index, item) in self.0.iter().enumerate() {
            let id = &item.id.0;
            if id.is_empty() || id.contains(char::is_whitespace) {
                return Err(format!("item id `{id}` must be one word"));
            }
            if item.max_count == 0 {
                return Err(format!("item `{id}`: max_count must be at least 1"));
            }
            let repeats = |other: &ItemDefinition| {
                other.id == item.id
                    || (other.kind == item.kind && !matches!(item.kind, ItemKind::Heal(_)))
            };
            if self.0[..index].iter().any(repeats) {
                return Err(format!(
                    "item `{id}` repeats an id, or a weapon or ammunition another item already is"
                ));
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &ItemId) -> Option<&ItemDefinition> {
        self.0.iter().find(|item| item.id == *id)
    }

    /// The reserve ammunition of `weapon`, if the game has any.
    pub fn ammo_for(&self, weapon: WeaponKind) -> Option<&ItemDefinition> {
        self.0
            .iter()
            .find(|item| item.kind == ItemKind::Ammo(weapon))
    }

    /// The item that is carrying `weapon`, if the weapon has to be found.
    pub fn weapon_item(&self, weapon: WeaponKind) -> Option<&ItemDefinition> {
        self.0
            .iter()
            .find(|item| item.kind == ItemKind::Weapon(weapon))
    }
}

/// So many of one item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u32,
}

/// On a player: what it carries, one stack per item, in the order they were first received.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// Most different items at once.
    slots: u32,
    /// Never holds an empty stack.
    stacks: Vec<ItemStack>,
}

/// On a player: what it last picked up from the world, for whoever shows or sounds it. A new
/// pickup is a higher `count`.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastPickup {
    /// How many ticks of this player's life in the session picked something up.
    pub count: u32,
    /// The kind of spot it came from; `None` for what the dead dropped.
    pub kind: Option<String>,
    /// What was taken, in the order it was taken.
    pub items: Vec<ItemStack>,
}

impl NetState for LastPickup {
    const ID: &'static str = "gameplay.last_pickup";
}

impl NetState for Inventory {
    const ID: &'static str = "gameplay.inventory";
}

impl Inventory {
    pub fn new(slots: u32) -> Self {
        Self {
            slots,
            stacks: Vec::new(),
        }
    }

    pub fn stacks(&self) -> &[ItemStack] {
        &self.stacks
    }

    pub fn count(&self, item: &ItemId) -> u32 {
        self.stacks
            .iter()
            .find(|stack| stack.item == *item)
            .map_or(0, |stack| stack.count)
    }

    /// How many more of `item` fit: up to its `max_count`, and none of a new item when every slot
    /// is taken.
    pub fn room_for(&self, item: &ItemDefinition) -> u32 {
        let held = self.count(&item.id);
        if held == 0 && self.stacks.len() >= self.slots as usize {
            return 0;
        }
        item.max_count.saturating_sub(held)
    }

    /// Adds up to `count`; returns how many were taken in.
    pub fn add(&mut self, item: &ItemDefinition, count: u32) -> u32 {
        let added = count.min(self.room_for(item));
        if added == 0 {
            return 0;
        }
        match self.stacks.iter_mut().find(|stack| stack.item == item.id) {
            Some(stack) => stack.count += added,
            None => self.stacks.push(ItemStack {
                item: item.id.clone(),
                count: added,
            }),
        }
        added
    }

    /// Removes up to `count`; returns how many there were to remove.
    pub fn take(&mut self, item: &ItemId, count: u32) -> u32 {
        let taken = count.min(self.count(item));
        for stack in &mut self.stacks {
            if stack.item == *item {
                stack.count -= taken;
            }
        }
        self.stacks.retain(|stack| stack.count > 0);
        taken
    }

    /// Empties it; returns what it held.
    pub fn clear(&mut self) -> Vec<ItemStack> {
        std::mem::take(&mut self.stacks)
    }
}

/// Whether a player with `inventory` carries `weapon`.
pub fn owns_weapon(
    inventory: Option<&Inventory>,
    catalog: &ItemCatalog,
    weapon: WeaponKind,
) -> bool {
    match (inventory, catalog.weapon_item(weapon)) {
        (Some(inventory), Some(item)) => inventory.count(&item.id) > 0,
        _ => true,
    }
}

/// Where a weapon's reserve ammunition comes from: the player's inventory, or nowhere in particular
/// (unlimited) without one or without such an item in the game.
pub struct AmmoSource<'a> {
    pub inventory: Option<&'a mut Inventory>,
    pub catalog: &'a ItemCatalog,
    pub weapon: WeaponKind,
}

impl AmmoSource<'_> {
    /// Rounds in reserve; `None` is unlimited.
    pub fn reserve(&self) -> Option<u32> {
        reserve_of(self.inventory.as_deref(), self.catalog, self.weapon)
    }

    /// Takes up to `wanted` rounds; returns how many came.
    pub fn take(&mut self, wanted: u32) -> u32 {
        match (&mut self.inventory, self.catalog.ammo_for(self.weapon)) {
            (Some(inventory), Some(item)) => inventory.take(&item.id, wanted),
            _ => wanted,
        }
    }
}

/// Rounds of `weapon` a player with `inventory` has in reserve; `None` is unlimited.
pub fn reserve_of(
    inventory: Option<&Inventory>,
    catalog: &ItemCatalog,
    weapon: WeaponKind,
) -> Option<u32> {
    Some(inventory?.count(&catalog.ammo_for(weapon)?.id))
}

/// "Give this player an item." Written in `GameplaySystems::Pickup` or by a command; the inventory
/// module adds what fits to a living player.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct GrantRequested {
    pub player: PlayerId,
    pub item: ItemId,
    pub count: u32,
}

/// A player died and lost everything it carried; read in `GameplaySystems::Drops`. Not sent when a
/// new match empties inventories.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ItemsLost {
    pub player: PlayerId,
    pub items: Vec<ItemStack>,
    /// The middle of the body when it died.
    pub at: Vec3,
}

/// "Restore this many hit points." Written for a used item; health acts on it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealRequested {
    pub player: PlayerId,
    pub amount: u32,
}
