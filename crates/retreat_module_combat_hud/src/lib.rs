//! Combat HUD of this machine's player: it reads the contract's state and changes only on-screen
//! widgets. Each element needs its data, so it works with any of the other modules absent:
//!
//! | Widget | Shown when |
//! |---|---|
//! | `hud_crosshair` | the player has a `WeaponStatus` and is alive |
//! | `hud_ammo` | the player has a `WeaponStatus` and is alive |
//! | `hud_hit` | for a moment after `WeaponStatus::hits_landed` rises |
//! | `hud_health` | the player has `Health` |
//! | `hud_respawn` | the player has a `RespawnTimer` |
//!
//! What is shown is the pure [`hud_view`]; where it goes is the engine's HUD layout. A server has
//! no player and shows nothing. Text can be checked with the engine's `assert.ui_text`, visibility
//! with `assert.hud_shown`. Numbers: [`CombatHudConfig`].

mod commands;
mod config;
mod show;
mod view;

use bevy::prelude::*;
use retreat_engine_api::{AppCommandExt, AppHudExt, HudAnchor, HudKind, HudWidget};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::CombatHudConfig;
pub use view::{HudInput, HudView, hud_view};

pub const CROSSHAIR_WIDGET: &str = "hud_crosshair";
pub const AMMO_WIDGET: &str = "hud_ammo";
pub const HIT_WIDGET: &str = "hud_hit";
pub const HEALTH_WIDGET: &str = "hud_health";
pub const RESPAWN_WIDGET: &str = "hud_respawn";

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
        let text = |id, anchor, font_size| HudWidget {
            id,
            anchor,
            kind: HudKind::Text { font_size },
            visible: false,
        };
        app.register_hud_widget(HudWidget {
            id: CROSSHAIR_WIDGET,
            anchor: HudAnchor::Center,
            kind: HudKind::Crosshair {
                size: config.crosshair_size,
            },
            visible: false,
        })
        .register_hud_widget(text(AMMO_WIDGET, HudAnchor::BottomRight, config.font_size))
        .register_hud_widget(text(HIT_WIDGET, HudAnchor::Center, config.font_size))
        .register_hud_widget(text(HEALTH_WIDGET, HudAnchor::BottomLeft, config.font_size))
        .register_hud_widget(text(RESPAWN_WIDGET, HudAnchor::Center, config.font_size))
        .init_resource::<show::HitMarker>()
        .register_command(
            "assert.hud_shown",
            "assert.hud_shown <widget> <true|false>",
            commands::assert_shown_command,
        )
        .add_systems(
            FixedUpdate,
            show::time_hit_marker.after(GameplaySystems::Respawned),
        )
        .add_systems(Update, show::show_hud);
    }
}
