//! Player spawning: one character per player, at a spawn point of the loaded map. Never add it
//! together with the `players` module (both spawn `player_<id>`).
//!
//! Two ways, chosen by [`PlayerSpawnConfig::spawn_on_join`]:
//! - **On join** (the default, the arena): the server spawns a character when a client joins
//!   (`NetClientJoined`) and removes it when it leaves (`NetClientLeft`, `RemovePhysicsBody`).
//!   Clients get it through the engine's replication.
//! - **Saved characters** (`spawn_on_join: false`, the crawler; `characters.rs`): a character
//!   appears when its player starts playing it (`CharacterEntered`, from the account module) at the
//!   place it was saved, carries `PlayingCharacter`, and its place is saved on `CharacterLeft`, on a
//!   disconnect (it leaves at once) and every `autosave_seconds`; a takeover hands it to the new
//!   connection. New characters start in the map's home area.
//! - **Without a network:** `spawn.solo <player>` spawns a character and makes it this machine's
//!   player (`SoloPlayer`).
//! - The character is `player_<id>` (a saved character's body is `character_<character id>_<n>`,
//!   numbered per appearance, because a body that left keeps its name for the engine's rollback
//!   window and a character may come back at once): `PlayerId`, `LookAngles` (facing the spawn point's way) and
//!   an upright capsule body. How it moves and looks is up to other modules; this one adds only a
//!   plain capsule mesh where the app draws meshes.
//! - The spawn point is the one farthest from the other characters.

mod characters;
mod commands;
mod spawn;

use bevy::prelude::*;
use retreat_engine_api::AppCommandExt;
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use characters::{PLACE_RECORD_PART, character_body_name, entry_place};
pub use spawn::{spawn_character, spawn_character_named};

/// Insert before adding the plugin to change it (the same on server and clients).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlayerSpawnConfig {
    /// Spawn a character when a client joins (the arena). Off: when its player starts playing a
    /// saved character (the crawler).
    pub spawn_on_join: bool,
    /// How often the places of played characters are saved (placeholder number).
    pub autosave_seconds: f32,
}

impl Default for PlayerSpawnConfig {
    fn default() -> Self {
        Self {
            spawn_on_join: true,
            autosave_seconds: 30.0,
        }
    }
}

/// Adds player spawning and the `spawn.*` commands.
pub struct PlayerSpawnModulePlugin;

impl Plugin for PlayerSpawnModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<PlayerSpawnConfig>();
        let config = *app.world().resource::<PlayerSpawnConfig>();
        if !(config.autosave_seconds.is_finite() && config.autosave_seconds > 0.0) {
            panic!(
                "invalid PlayerSpawnConfig: autosave_seconds must be more than 0, got {}",
                config.autosave_seconds
            );
        }
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
            );
        if config.spawn_on_join {
            app.add_systems(
                FixedUpdate,
                // Before health: a new character gets its health in the tick it appears.
                (spawn::remove_left_clients, spawn::spawn_joined_clients)
                    .chain()
                    .in_set(GameplaySystems::Act),
            );
        } else {
            app.add_systems(
                FixedUpdate,
                (
                    characters::handle_disconnected_characters,
                    characters::remove_left_characters,
                    characters::spawn_entered_characters,
                    characters::autosave_places,
                )
                    .chain()
                    .in_set(GameplaySystems::Act),
            );
        }
    }
}
