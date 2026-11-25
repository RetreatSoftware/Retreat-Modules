//! How player characters look. Render only: it derives everything from shared state on each machine
//! and changes nothing in the simulation, so it works with any module that creates `PlayerId`
//! entities and with no movement module at all.
//!
//! For every player, each frame:
//! - [`CharacterVisual`]: the configured model (or the engine's plain shape), turned to where the
//!   character moves (it keeps that facing when it stops; before it ever moved, where it looks),
//!   playing the clip for what it does ([`locomotion_of`]: idle, walk, run, air);
//! - [`EntityColor`]: one color per player id;
//! - [`RenderSmoothing`] on every player but this machine's own (none on a server, which has no
//!   player).
//!
//! The model, its clips, the colors and the thresholds are [`CharacterVisualsConfig`]; a game sets
//! them. The default is the plain shape.

mod commands;
mod config;
mod dress;
mod locomotion;

use bevy::prelude::*;
use retreat_engine_api::AppCommandExt;
use retreat_gameplay_api::add_gameplay_contract;

pub use config::{CharacterModel, CharacterVisualsConfig, Clip};
pub use dress::VisualMotion;
pub use locomotion::{Locomotion, clips_for, facing_of, locomotion_of};

/// Adds the looks. Insert a [`CharacterVisualsConfig`] first to choose a model and colors.
pub struct CharacterVisualsModulePlugin;

impl Plugin for CharacterVisualsModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<CharacterVisualsConfig>();
        if let Err(problem) = app.world().resource::<CharacterVisualsConfig>().validate() {
            panic!("invalid CharacterVisualsConfig: {problem}");
        }
        app.register_command(
            "assert.visuals_state",
            "assert.visuals_state <player> <idle|walk|run|air>",
            commands::assert_state_command,
        )
        .register_command(
            "assert.visuals_facing",
            "assert.visuals_facing <player> <yaw> <tolerance>",
            commands::assert_facing_command,
        )
        .register_command(
            "assert.visuals_color",
            "assert.visuals_color <player> <color number>",
            commands::assert_color_command,
        )
        // After the fixed ticks of the frame and before the engine draws (`Update`).
        .add_systems(PreUpdate, dress::dress_players);
    }
}
