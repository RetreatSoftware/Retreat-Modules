//! Inventory: what every player carries. The server decides; clients run the same systems as
//! prediction and are corrected by the engine.
//!
//! - Every `PlayerId` entity gets an `Inventory` holding the starting loadout.
//! - `GrantRequested` (a pickup, or `inventory.grant`) adds what fits to a living player.
//! - `inventory.use <item>` (client allowed) uses one of a usable item: a health kit writes
//!   `HealRequested`. Refused for the dead, in a finished match, without the item, and when it
//!   would do nothing (full health). `inventory_use` (E) uses the first usable item carried.
//! - A death empties the inventory and says what was lost (`ItemsLost`, where).
//! - A new life (`PlayerRespawned`) and a new match (`MatchRestarted`) reset it to the loadout.
//!
//! Items are the game's `ItemCatalog`; ammunition is taken out by the weapon modules. Numbers:
//! [`InventoryConfig`] and the catalog. Rolled-back state: the contract's `Inventory`.

mod commands;
mod config;
mod input;
mod systems;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, AppNetExt, HELD_ONLY, InputAction, InputActionSystems, InputKey,
};
use retreat_gameplay_api::{GameplaySystems, ItemCatalog, add_gameplay_contract};

pub use config::InventoryConfig;

/// Adds the inventory. Insert the `ItemCatalog` and an [`InventoryConfig`] first.
pub struct InventoryModulePlugin;

impl Plugin for InventoryModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<InventoryConfig>();
        let (config, catalog) = (
            app.world().resource::<InventoryConfig>(),
            app.world().resource::<ItemCatalog>(),
        );
        if let Err(problem) = config.validate(catalog) {
            panic!("invalid InventoryConfig: {problem}");
        }
        app.register_command(
            "inventory.grant",
            "inventory.grant <player> <item> <count>",
            commands::grant_command,
        )
        .register_command(
            "inventory.use",
            "inventory.use <item>",
            commands::use_command,
        )
        .allow_client_command("inventory.use")
        .register_command(
            "assert.inventory",
            "assert.inventory <player> <item> <count>",
            commands::assert_command,
        )
        .register_action(InputAction {
            id: "inventory_use",
            command_line: HELD_ONLY,
            default_keys: &[InputKey::E],
        })
        .add_systems(
            FixedUpdate,
            (
                (systems::attach_inventories, input::send_input)
                    .chain()
                    .after(InputActionSystems)
                    .before(GameplaySystems::Act),
                (systems::apply_grants, systems::lose_on_death)
                    .chain()
                    .in_set(GameplaySystems::Items),
                systems::reset_on_restart.in_set(GameplaySystems::Restart),
                systems::reset_on_respawn.in_set(GameplaySystems::Respawned),
            ),
        );
    }
}
