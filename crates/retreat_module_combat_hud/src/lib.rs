//! Combat HUD of this machine's player: it reads the contract's state and changes only on-screen
//! widgets. Each element needs its data, so it works with any of the other modules absent:
//!
//! | Widget | Shown when |
//! |---|---|
//! | `hud_crosshair` | the player carries a weapon and is alive |
//! | `hud_ammo_<weapon>` | the same; one line for each weapon in `Weapons`, with its name and ammo |
//! | `hud_hit` | for a moment after any weapon's `hits_landed` rises |
//! | `hud_health` | the player has `Health` |
//! | `hud_respawn` | the player has a `RespawnTimer` |
//! | `hud_match_clock` | there is a match: time left, or the countdown to the next one |
//! | `hud_match_banner` | the match is finished: the winner or a draw |
//! | `hud_match_score_head`, `hud_match_score_row<n>` | the match is finished, or `hud_scoreboard` (Tab) is held or `hud.scoreboard on`; one row per player, best first |
//! | `hud_kill<n>` | for a while after a death appears in the `KillLog` |
//!
//! While the scoreboard is shown, the middle of the screen (crosshair, hit marker, respawn) is
//! empty. What is shown is the pure [`hud_view`] and [`match_view`]; where it goes is the engine's
//! HUD layout. A server has no player and shows nothing. Names come from the contract's
//! `PlayerNames`. Visibility is checked with `assert.hud_shown`, text with `assert.hud_text` (runs
//! of spaces count as one; the engine's `assert.ui_text` cannot compare columns). Numbers:
//! [`CombatHudConfig`].

mod commands;
mod config;
mod match_view;
mod show;
mod view;
mod widgets;

use bevy::prelude::*;
use retreat_engine_api::{AppCommandExt, AppInputExt, HELD_ONLY, InputAction, InputKey};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::CombatHudConfig;
pub use match_view::{KillFeed, MatchInput, MatchView, kill_line, match_view};
pub use view::{HudInput, HudView, hud_view};
pub use widgets::{
    CLOCK_WIDGET, CROSSHAIR_WIDGET, HEALTH_WIDGET, HIT_WIDGET, HudLines, RESPAWN_WIDGET,
    RESULT_WIDGET, SCORE_HEAD_WIDGET, ammo_widget,
};

/// Adds the HUD. Insert a [`CombatHudConfig`] first to change the numbers.
pub struct CombatHudModulePlugin;

impl Plugin for CombatHudModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<CombatHudConfig>();
        if let Err(problem) = app.world().resource::<CombatHudConfig>().validate() {
            panic!("invalid CombatHudConfig: {problem}");
        }
        let config = app.world().resource::<CombatHudConfig>().clone();
        widgets::register_widgets(app, &config);
        app.init_resource::<show::HitMarker>()
            .init_resource::<show::ShownKills>()
            .init_resource::<show::ScoreboardSwitch>()
            .register_command(
                "hud.scoreboard",
                "hud.scoreboard <on|off>",
                commands::scoreboard_command,
            )
            .register_command(
                "assert.hud_shown",
                "assert.hud_shown <widget> <true|false>",
                commands::assert_shown_command,
            )
            .register_command(
                "assert.hud_text",
                "assert.hud_text <widget> <words...>",
                commands::assert_text_command,
            )
            .register_action(InputAction {
                id: "hud_scoreboard",
                command_line: HELD_ONLY,
                default_keys: &[InputKey::Tab],
            })
            .add_systems(
                FixedUpdate,
                (show::time_hit_marker, show::time_kill_feed).after(GameplaySystems::Respawned),
            )
            .add_systems(Update, show::show_hud);
    }
}
