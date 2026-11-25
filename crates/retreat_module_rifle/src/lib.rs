//! Hitscan rifle for every player: fire rate, magazine, reload, hits.
//!
//! One shot:
//! 1. `send_input` (this machine's player only): while `rifle_fire` is held and the weapon is ready,
//!    it aims from the eyes at what the center of the camera shows and issues
//!    `rifle.fire <yaw> <pitch> <view_tick>`. A network client queues the line, which the engine
//!    runs on the client (prediction) and on the server.
//! 2. The command checks that the player may shoot (alive, match running, ready), uses a round and
//!    writes `PlayerFired`.
//! 3. Only where the command is authoritative (a server or a game without a network) it casts the
//!    ray against the other players **as the shooter saw them** (`cast_ray_at`) and writes
//!    `DamageRequested`. Health decides what the damage does.
//! 4. `DamageApplied` of a shot raises the shooter's `WeaponStatus::hits_landed`, which reaches the
//!    shooter with the replicated state (a HUD shows a hit marker).
//!
//! The dead neither shoot nor stop a shot. Ammo is refilled on `PlayerRespawned`.
//! Numbers: [`RifleConfig`]. Rolled-back state: `WeaponStatus` (contract) and [`RifleState`].

mod aim;
mod commands;
mod config;
mod input;
mod rules;
mod state;
mod targets;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, AppNetExt, AppNetStateExt, HELD_ONLY, InputAction,
    InputActionSystems, InputKey,
};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use aim::aim_angles;
pub use config::RifleConfig;
pub use rules::{LATE_INPUT_TOLERANCE_TICKS, RifleTicks, ready_to_send, try_fire, try_reload};
pub use state::RifleState;

/// Adds the rifle. Insert a [`RifleConfig`] first to change the numbers.
pub struct RifleModulePlugin;

impl Plugin for RifleModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<RifleConfig>();
        if let Err(problem) = app.world().resource::<RifleConfig>().validate() {
            panic!("invalid RifleConfig: {problem}");
        }
        app.register_net_state::<RifleState>()
            .register_command(
                "rifle.fire",
                "rifle.fire <yaw> <pitch> [view_tick]",
                commands::fire_command,
            )
            .register_command("rifle.reload", "rifle.reload", commands::reload_command)
            .allow_client_command("rifle.fire")
            .allow_client_command("rifle.reload")
            .register_command(
                "assert.rifle_ammo",
                "assert.rifle_ammo <player> <rounds>",
                commands::assert_ammo_command,
            )
            .register_command(
                "assert.rifle_hits",
                "assert.rifle_hits <player> <hits>",
                commands::assert_hits_command,
            )
            .register_action(held("rifle_fire", &[InputKey::MouseLeft]))
            .register_action(held("rifle_reload", &[InputKey::R]))
            .add_systems(
                FixedUpdate,
                (
                    // Before the timers, so a shot without a network is in this tick's rules.
                    input::send_input
                        .after(InputActionSystems)
                        .before(GameplaySystems::Act),
                    (state::attach_weapons, state::advance_timers)
                        .chain()
                        .in_set(GameplaySystems::Act),
                    state::count_hits.in_set(GameplaySystems::Match),
                    state::refill_respawned.in_set(GameplaySystems::Respawned),
                ),
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
