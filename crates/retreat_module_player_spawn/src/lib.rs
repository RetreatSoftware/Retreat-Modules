//! Player spawning: one character per player, at a spawn point of the loaded map. Never add it
//! together with the `players` module (both spawn `player_<id>`).
//!
//! - **Server:** `NetClientJoined` spawns the character, `NetClientLeft` removes it
//!   (`RemovePhysicsBody`). Clients get it through the engine's replication.
//! - **Without a network:** `spawn.solo <player>` spawns a character and makes it this machine's
//!   player (`SoloPlayer`).
//! - The character is `player_<id>`: `PlayerId`, `LookAngles` (facing the spawn point's way) and
//!   an upright capsule body. How it moves and looks is up to other modules; this one adds only a
//!   plain capsule mesh where the app draws meshes.
//! - The spawn point is the one farthest from the other characters.

mod commands;
mod spawn;

use bevy::prelude::*;
use retreat_engine_api::AppCommandExt;
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use spawn::spawn_character;

/// Adds player spawning and the `spawn.*` commands.
pub struct PlayerSpawnModulePlugin;

impl Plugin for PlayerSpawnModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.register_command("spawn.join", "spawn.join <player>", commands::join_command)
            .register_command("spawn.solo", "spawn.solo <player>", commands::solo_command)
            .register_command(
                "spawn.leave",
                "spawn.leave <player>",
                commands::leave_command,
            )
            .register_command(
                "assert.spawn_count",
                "assert.spawn_count <characters>",
                commands::assert_count_command,
            )
            .add_systems(
                FixedUpdate,
                // Before health: a new character gets its health in the tick it appears.
                (spawn::remove_left_clients, spawn::spawn_joined_clients)
                    .chain()
                    .in_set(GameplaySystems::Act),
            );
    }
}
