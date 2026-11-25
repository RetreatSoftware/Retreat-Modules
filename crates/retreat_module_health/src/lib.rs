//! Health module: damage, death, spawn protection and respawn. The server decides; clients run the
//! same systems as prediction and are corrected by the engine.
//!
//! - Every `PlayerId` entity gets `Health` and spawn protection. Firing (`PlayerFired`) ends the
//!   protection at once.
//! - `DamageRequested` removes hit points unless the target is dead or protected, or the match is
//!   finished. The last point sends `PlayerDied` and starts the respawn timer.
//! - After the delay, and for everyone on `MatchRestarted`, a player has full health again and
//!   `PlayerRespawned` names the spawn point: the one farthest from the nearest living player.
//!   Without spawn points (no map) the player respawns where it is.
//! - A living player below the fall-out height dies (cause `World`).
//! - Moving a character to its spawn point belongs to the module that owns the character (it reads
//!   `PlayerRespawned`). This module moves only its own dummies ([`HealthDummy`]), which have no
//!   physics body.
//!
//! Numbers: [`HealthConfig`]. All state is in `NetState` components, so it rolls back and
//! replicates.

mod commands;
mod config;
mod damage;
mod respawn;
mod state;

use bevy::prelude::*;
use retreat_engine_api::{AppCommandExt, AppNetStateExt};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use commands::HealthDummy;
pub use config::HealthConfig;
pub use retreat_gameplay_api::RespawnTimer;
pub use state::SpawnProtection;

/// Adds the health rules and commands. Insert a [`HealthConfig`] first to change the numbers.
pub struct HealthModulePlugin;

impl Plugin for HealthModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<HealthConfig>();
        if let Err(problem) = app.world().resource::<HealthConfig>().validate() {
            panic!("invalid HealthConfig: {problem}");
        }
        app.add_message::<damage::KillRequested>()
            .register_net_state::<SpawnProtection>()
            .register_net_state::<HealthDummy>()
            .register_command(
                "health.add_dummy",
                "health.add_dummy <player>",
                commands::add_dummy_command,
            )
            .register_command(
                "health.damage",
                "health.damage <player> <amount> [attacker]",
                commands::damage_command,
            )
            .register_command(
                "health.kill",
                "health.kill <player>",
                commands::kill_command,
            )
            .register_command(
                "assert.health",
                "assert.health <player> <hit points>",
                commands::assert_health_command,
            )
            .register_command(
                "assert.health_dead",
                "assert.health_dead <player>",
                commands::assert_dead_command,
            )
            .add_systems(
                FixedUpdate,
                (
                    // Timers first, so a timer started in this tick is not counted in it.
                    (
                        respawn::advance_timers,
                        respawn::attach_health,
                        respawn::end_protection_on_fire,
                        damage::request_fall_outs,
                        damage::apply_kills,
                        damage::apply_damage,
                    )
                        .chain()
                        .in_set(GameplaySystems::Damage),
                    respawn::respawn_all_on_restart.in_set(GameplaySystems::Restart),
                    commands::move_dummies_to_spawn.in_set(GameplaySystems::Respawned),
                ),
            );
    }
}
