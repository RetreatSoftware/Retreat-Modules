//! Match module: a timed free-for-all. The server decides; clients run the same system as
//! prediction and are corrected by the engine.
//!
//! - The match is one entity named `match` with the contract's `MatchState` and `Scoreboard`. It is
//!   created when the first player (`PlayerId`) appears, or by `match.start`.
//! - The scoreboard has one line per present player.
//! - While running, each `PlayerDied` scores (a kill of another player, or a penalty for a
//!   self-kill or a world death) and the clock counts down. At zero the match is finished and
//!   `MatchEnded` names the winner (`None` is a draw).
//! - A finished match shows its result for a while, then restarts: scores are cleared and
//!   `MatchRestarted` is sent, on which other modules reset themselves.
//!
//! Without a health module no deaths arrive; the timer still runs. Numbers: [`MatchConfig`].

mod commands;
mod config;
mod lifecycle;
mod rules;

use bevy::prelude::*;
use retreat_engine_api::AppCommandExt;
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::MatchConfig;
pub use rules::{score_death, scoreboard_for};

/// Adds the match rules and commands. Insert a [`MatchConfig`] first to change the numbers.
pub struct MatchModulePlugin;

impl Plugin for MatchModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<MatchConfig>();
        if let Err(problem) = app.world().resource::<MatchConfig>().validate() {
            panic!("invalid MatchConfig: {problem}");
        }
        app.register_command("match.start", "match.start", commands::start_command)
            .register_command("match.end", "match.end", commands::end_command)
            .register_command("match.restart", "match.restart", commands::restart_command)
            .register_command(
                "assert.match_running",
                "assert.match_running",
                commands::assert_running_command,
            )
            .register_command(
                "assert.match_finished",
                "assert.match_finished",
                commands::assert_finished_command,
            )
            .register_command(
                "assert.match_score",
                "assert.match_score <player> <points>",
                commands::assert_score_command,
            )
            .register_command(
                "assert.match_winner",
                "assert.match_winner <player|draw>",
                commands::assert_winner_command,
            )
            .add_systems(
                FixedUpdate,
                // In this order: a death in the last tick still counts.
                (
                    lifecycle::start_with_first_player,
                    lifecycle::score_and_advance,
                )
                    .chain()
                    .in_set(GameplaySystems::Match),
            );
    }
}
