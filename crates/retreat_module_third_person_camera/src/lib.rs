//! Third-person camera: looks over a player's shoulder along the player's `LookAngles`. It only
//! edits a `GameCamera`; no game state.
//!
//! - `shoulder.follow <camera> [player]`: without a player it follows this machine's player, as soon
//!   as there is one. `shoulder.stop <camera>` leaves the camera where it is.
//! - The camera moves in front of anything between the head and its place (two ball casts: to the
//!   shoulder, then back), so it never looks through a wall. Characters do not block it.
//! - It is placed after each fixed tick (`FixedPostUpdate`), not during a client's replay.
//!
//! Numbers: [`ShoulderConfig`]. Mouse look itself belongs to the module that turns the player.

mod commands;
mod config;
mod follow;
mod placement;

use bevy::prelude::*;
use retreat_engine_api::{AppCommandExt, AppInputExt, InputAction, InputKey};
use retreat_gameplay_api::add_gameplay_contract;

pub use config::ShoulderConfig;
pub use follow::ShoulderCamera;
pub use placement::{ShoulderPoints, shoulder_points};

/// Adds the camera. Insert a [`ShoulderConfig`] first to change the numbers.
pub struct ThirdPersonCameraModulePlugin;

impl Plugin for ThirdPersonCameraModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<ShoulderConfig>();
        if let Err(problem) = app.world().resource::<ShoulderConfig>().validate() {
            panic!("invalid ShoulderConfig: {problem}");
        }
        app.register_command(
            "shoulder.follow",
            "shoulder.follow <camera> [player]",
            commands::follow_command,
        )
        .register_command(
            "shoulder.stop",
            "shoulder.stop <camera>",
            commands::stop_command,
        )
        .register_command("shoulder.swap", "shoulder.swap", commands::swap_command)
        .register_command(
            "assert.shoulder_distance",
            "assert.shoulder_distance <camera> <least metres> <most metres>",
            commands::assert_distance_command,
        )
        .register_action(InputAction {
            id: "shoulder_swap",
            command_line: "shoulder.swap",
            default_keys: &[InputKey::Q],
        })
        .add_systems(FixedPostUpdate, follow::place_cameras);
    }
}
