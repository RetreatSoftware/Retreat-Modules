//! The module's numbers.

use bevy::prelude::*;
use retreat_gameplay_api::{Inventory, ItemCatalog, ItemId, SoundCue, validate_cues};

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct InventoryConfig {
    /// Most different items a player carries.
    pub slots: u32,
    /// What every new life starts with, in this order.
    pub loadout: Vec<(ItemId, u32)>,
    /// Heard when an item is used, where its user is.
    pub use_sound: Option<SoundCue>,
}

impl Default for InventoryConfig {
    fn default() -> Self {
        Self {
            slots: 4,
            loadout: Vec::new(),
            use_sound: None,
        }
    }
}

impl InventoryConfig {
    /// What is wrong, naming the field and the item.
    pub fn validate(&self, catalog: &ItemCatalog) -> Result<(), String> {
        validate_cues([&self.use_sound])?;
        if self.slots == 0 {
            return Err("slots must be at least 1".to_string());
        }
        let mut start = Inventory::new(self.slots);
        for (item, count) in &self.loadout {
            let Some(definition) = catalog.get(item) else {
                return Err(format!("loadout: the catalog has no item `{item}`"));
            };
            if start.add(definition, *count) != *count || *count == 0 {
                return Err(format!(
                    "loadout: {count} of `{item}` does not fit (its max_count, or the slots)"
                ));
            }
        }
        Ok(())
    }

    /// The inventory of a new life.
    pub fn starting(&self, catalog: &ItemCatalog) -> Inventory {
        let mut start = Inventory::new(self.slots);
        for (item, count) in &self.loadout {
            if let Some(definition) = catalog.get(item) {
                start.add(definition, *count);
            }
        }
        start
    }
}
