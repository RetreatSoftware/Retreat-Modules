//! Character movement: walk, sprint, jump and dash for every player with an upright character body
//! ([`retreat_engine_api::PhysicsBodyKind::Character`]). Server-authoritative and predicted.
//!
//! One tick:
//! 1. `send_input` (this machine's player only) turns held keys, axes and mouse movement into the
//!    tick's [`PlayerInput`]. A network client queues it as a `movement.input` line, which the
//!    engine runs on the client (prediction), on the server and in replays. Without a network it is
//!    stored directly.
//! 2. `move_characters` (every player, in id order) sets the body's velocity with the pure rule
//!    [`step_movement`]. The engine's stepping (`CharacterStepping`) then walks it over stairs and
//!    slopes and reports the ground. The dead do not steer.
//! 3. A player that respawned (`PlayerRespawned`) is teleported to its spawn point, facing its way.
//!
//! All three run after the gameplay sets, so the tick's spawns, deaths and respawns are known.
//! Numbers: [`MovementConfig`].

mod commands;
mod config;
mod input;
mod motion;
mod respawn;
mod simulate;
mod sounds;
mod state;

use bevy::prelude::*;
use retreat_engine_api::{
    AppAxisExt, AppCommandExt, AppInputExt, AppNetExt, AppNetStateExt, HELD_ONLY, InputAction,
    InputAxis, InputKey, PhysicsControlSystems,
};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::MovementConfig;
pub use input::LocalAim;
pub use motion::{MovementStep, step_movement};
pub use state::{MovementState, PlayerInput};

/// The axis that is -1 left to 1 right.
pub const STRAFE_AXIS: &str = "movement_strafe";
/// The axis that is -1 back to 1 forward.
pub const ADVANCE_AXIS: &str = "movement_advance";

/// Adds character movement. Insert a [`MovementConfig`] first to change the numbers.
pub struct CharacterMovementModulePlugin;

impl Plugin for CharacterMovementModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<MovementConfig>();
        if let Err(problem) = app.world().resource::<MovementConfig>().validate() {
            panic!("invalid MovementConfig: {problem}");
        }
        app.init_resource::<LocalAim>()
            .register_net_state::<MovementState>()
            .register_command(
                "movement.input",
                "movement.input <strafe> <advance> <yaw> <pitch> <sprint> <jump> <dash>",
                commands::input_command,
            )
            .allow_client_command("movement.input")
            .register_command(
                "movement.look",
                "movement.look <yaw> <pitch>",
                commands::look_command,
            )
            .register_command(
                "assert.movement_grounded",
                "assert.movement_grounded <player> <true|false>",
                commands::assert_grounded_command,
            )
            .register_command(
                "assert.movement_speed",
                "assert.movement_speed <player> <metres per second> <tolerance>",
                commands::assert_speed_command,
            )
            .register_command(
                "assert.movement_near",
                "assert.movement_near <player> <feet x> <feet y> <feet z> <tolerance>",
                commands::assert_near_command,
            )
            .register_command(
                "assert.movement_look",
                "assert.movement_look <player> <yaw> <tolerance>",
                commands::assert_look_command,
            )
            .register_action(held("movement_forward", &[InputKey::W]))
            .register_action(held("movement_back", &[InputKey::S]))
            .register_action(held("movement_left", &[InputKey::A]))
            .register_action(held("movement_right", &[InputKey::D]))
            .register_action(held("movement_sprint", &[InputKey::ShiftLeft]))
            .register_action(held("movement_jump", &[InputKey::Space]))
            .register_action(held("movement_dash", &[InputKey::ControlLeft]))
            // Mouse look needs the cursor: a click takes it, Escape gives it back.
            .register_action(InputAction {
                id: "movement_capture_cursor",
                command_line: "input.capture_cursor",
                default_keys: &[InputKey::MouseLeft],
            })
            .register_action(InputAction {
                id: "movement_release_cursor",
                command_line: "input.release_cursor",
                default_keys: &[InputKey::Escape],
            })
            .register_axis(InputAxis {
                id: STRAFE_AXIS,
                negative_action: "movement_left",
                positive_action: "movement_right",
            })
            .register_axis(InputAxis {
                id: ADVANCE_AXIS,
                negative_action: "movement_back",
                positive_action: "movement_forward",
            })
            .add_systems(
                FixedUpdate,
                (
                    state::attach_state,
                    input::send_input,
                    simulate::move_characters.in_set(PhysicsControlSystems),
                    respawn::teleport_respawned.in_set(PhysicsControlSystems),
                    sounds::play_movement_sounds,
                )
                    .chain()
                    // Spawns, deaths and respawns of this tick are decided (the sets also run
                    // after the tick's input).
                    .after(GameplaySystems::Respawned),
            );
    }
}

fn held(id: &'static str, default_keys: &'static [InputKey]) -> InputAction {
    InputAction {
        id,
        command_line: HELD_ONLY,
        default_keys,
    }
}
