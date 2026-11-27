//! Loot: things on the ground. The server decides everything; clients are told (the pickups are
//! shared state), count their timers down and draw them.
//!
//! - At every spot of the map's `PickupSpots` whose kind the config knows stands a pickup
//!   (`pickup/<spot>`). Taken, it comes back after its kind's delay.
//! - Walking over a pickup (horizontally within `reach`, on its ground) takes it if the player lives
//!   and can store at least one of its items (`Inventory::room_for`): what fits is granted
//!   (`GrantRequested`). Players are served in id order, pickups in name order.
//! - What a player loses on death (`ItemsLost`) lies there (`drop/<player>/<n>`) for a while; a
//!   taker gets what fits and the rest stays.
//! - What a player took is written to its `LastPickup`; every machine plays that kind's sound.
//! - A new match puts every spot back and removes the drops. Nothing is picked up while a match
//!   shows its result.
//!
//! Without an inventory module nobody can take anything. Numbers and contents: [`LootConfig`].
//! Rolled-back state: [`Pickup`] and `NetPose` per pickup.

mod commands;
mod config;
mod looks;
mod sounds;
mod state;
mod systems;

use bevy::prelude::*;
use retreat_engine_api::{AppCommandExt, AppNetStateExt};
use retreat_gameplay_api::{GameplaySystems, ItemCatalog, add_gameplay_contract};

pub use config::{LootConfig, PickupKind, PickupLook};
pub use state::{Pickup, PickupOrigin};
pub use systems::HOVER_HEIGHT;

/// Adds the loot. Insert the `ItemCatalog` and a [`LootConfig`] first.
pub struct LootModulePlugin;

impl Plugin for LootModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<LootConfig>();
        let (config, catalog) = (
            app.world().resource::<LootConfig>(),
            app.world().resource::<ItemCatalog>(),
        );
        if let Err(problem) = config.validate(catalog) {
            panic!("invalid LootConfig: {problem}");
        }
        app.register_net_state::<Pickup>()
            .register_command(
                "assert.loot_available",
                "assert.loot_available <spot> <true|false>",
                commands::assert_available_command,
            )
            .register_command(
                "assert.loot_drops",
                "assert.loot_drops <count>",
                commands::assert_drops_command,
            )
            .add_systems(
                FixedUpdate,
                (
                    (
                        systems::attach_records,
                        systems::place_spot_pickups,
                        systems::advance_timers,
                        systems::take_pickups,
                        sounds::play_pickup_sounds,
                    )
                        .chain()
                        .in_set(GameplaySystems::Pickup),
                    systems::drop_lost_items.in_set(GameplaySystems::Drops),
                    systems::reset_on_restart.in_set(GameplaySystems::Restart),
                ),
            )
            .add_systems(Update, looks::dress_pickups);
    }
}
