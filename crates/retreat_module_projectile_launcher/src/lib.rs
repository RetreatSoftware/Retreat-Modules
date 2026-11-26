//! Rocket launcher for every player: fire rate, magazine, reload, flying rockets, blast damage.
//!
//! One rocket:
//! 1. `send_input` (this machine's player only): while `launcher_fire` is held and the weapon is
//!    ready it aims at what the center of the camera shows and issues `launcher.fire <yaw> <pitch>`.
//! 2. The command runs on the shooter's client at once (prediction) and on the server. It uses a
//!    round, writes `PlayerFired` and creates the rocket at the eyes with `spawn_predicted`, so the
//!    engine merges both entities.
//! 3. `fly_rockets` moves every rocket each tick and explodes it on what it meets (not its owner,
//!    not the dead) or at the end of its life. There is no lag compensation: shooters lead.
//! 4. Only where the simulation is authoritative (a server or a game without a network) an explosion
//!    writes `DamageRequested`: full on a direct hit, falling off with the distance to the body
//!    ([`blast_damage`]), nothing behind a wall, reduced for the owner. Health decides what it does.
//! 5. The explosion's look is one `VisualEffect` (shown once, replays included); the rocket stays a
//!    few ticks, then is removed. `MatchRestarted` removes every rocket.
//!
//! Numbers: [`LauncherConfig`]. Rolled-back state: the rifle-independent entry in the contract's
//! `Weapons`, [`LauncherState`] and one [`Rocket`] plus `NetPose` per rocket.

mod commands;
mod config;
mod flight;
mod input;
mod looks;
mod state;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, AppNetExt, AppNetStateExt, HELD_ONLY, InputAction,
    InputActionSystems, InputKey, VisualEffects,
};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::LauncherConfig;
pub use flight::{BlastPart, LINGER_TICKS, blast_damage, blast_look, distance_to_body};
pub use state::{LauncherState, Rocket};

/// Adds the launcher. Insert a [`LauncherConfig`] first to change the numbers.
pub struct ProjectileLauncherModulePlugin;

impl Plugin for ProjectileLauncherModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<LauncherConfig>();
        if let Err(problem) = app.world().resource::<LauncherConfig>().validate() {
            panic!("invalid LauncherConfig: {problem}");
        }
        app.init_resource::<VisualEffects>()
            .register_net_state::<LauncherState>()
            .register_net_state::<Rocket>()
            .register_command(
                "launcher.fire",
                "launcher.fire <yaw> <pitch>",
                commands::fire_command,
            )
            .register_command(
                "launcher.reload",
                "launcher.reload",
                commands::reload_command,
            )
            .allow_client_command("launcher.fire")
            .allow_client_command("launcher.reload")
            .register_command(
                "assert.launcher_ammo",
                "assert.launcher_ammo <player> <rockets>",
                commands::assert_ammo_command,
            )
            .register_command(
                "assert.launcher_hits",
                "assert.launcher_hits <player> <hits>",
                commands::assert_hits_command,
            )
            .register_command(
                "assert.launcher_rockets",
                "assert.launcher_rockets <flying>",
                commands::assert_rockets_command,
            )
            .register_action(held("launcher_fire", &[InputKey::MouseRight]))
            .register_action(held("launcher_reload", &[InputKey::R]))
            .add_systems(
                FixedUpdate,
                (
                    // Before the timers, so a shot without a network is in this tick's rules.
                    input::send_input
                        .after(InputActionSystems)
                        .before(GameplaySystems::Act),
                    (
                        state::attach_launchers,
                        state::advance_timers,
                        flight::fly_rockets,
                    )
                        .chain()
                        .in_set(GameplaySystems::Act),
                    state::count_hits.in_set(GameplaySystems::Match),
                    state::remove_rockets.in_set(GameplaySystems::Restart),
                    state::play_own_reload.after(GameplaySystems::Respawned),
                    state::refill_respawned.in_set(GameplaySystems::Respawned),
                ),
            )
            .add_systems(Update, looks::dress_rockets);
    }
}

fn held(id: &'static str, default_keys: &'static [InputKey]) -> InputAction {
    InputAction {
        id,
        command_line: HELD_ONLY,
        default_keys,
    }
}
