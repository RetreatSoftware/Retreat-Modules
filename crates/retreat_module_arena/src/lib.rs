//! Arena module: maps as data, built from fixed physics boxes.
//!
//! - `arena.load <map>` spawns every piece of the map as a fixed `PhysicsBody` named
//!   `arena/<piece>` and fills the contract resources `SpawnPoints` and `PickupSpots`. One map per session: a second
//!   load is an error (removed bodies keep their names for the engine's rollback window, so
//!   switching maps is not built).
//! - Every machine (server and clients) loads the map itself; fixed bodies are not replicated.
//! - A map is refused when a spawn point would put a character inside geometry, outside the arena
//!   or in the air ([`validate_map`]).
//!
//! - A map may have **areas** (`MapAreas`: the crawler's city and dungeons, with rules), filled on
//!   load; every spawn point then lies in an area and the home area has one.
//!
//! Maps: `graybox` (the arena), `crawler_city` (placeholder crawler city plus a placeholder
//! dungeon room far away, two areas).
//!
//! To add a map: write a `<name>_map.rs` returning [`ArenaMap`] and list it in [`MAPS`].

mod crawler_city_map;
mod floor;
mod graybox_map;
mod load_command;
mod map;
mod source_units;

use bevy::prelude::*;
use retreat_engine_api::AppCommandExt;
use retreat_gameplay_api::add_gameplay_contract;

pub use crawler_city_map::crawler_city_map;
pub use graybox_map::graybox_map;
pub use load_command::{ArenaPiece, LoadedArena, piece_entity_name};
pub use map::{ArenaMap, MapPiece, PieceKind, spawn_point_problem, validate_map};
pub use source_units::{retreat_to_source_cm, source_cm_to_retreat};

/// Builds one map.
pub type MapBuilder = fn() -> ArenaMap;

/// Every loadable map: its id and the function that builds it.
pub const MAPS: &[(&str, MapBuilder)] =
    &[("graybox", graybox_map), ("crawler_city", crawler_city_map)];

/// Adds `arena.load`.
pub struct ArenaModulePlugin;

impl Plugin for ArenaModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.register_command("arena.load", "arena.load <map>", load_command::load_command);
    }
}
