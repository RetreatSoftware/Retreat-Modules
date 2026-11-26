//! The HUD's widgets. The engine stacks widgets of one corner in id order, so the ids of a group
//! sort in the order they are read. Lines that exist once per something are made from that count.

use crate::config::CombatHudConfig;
use bevy::prelude::*;
use retreat_engine_api::{AppHudExt, HudAnchor, HudKind, HudWidget};
use retreat_gameplay_api::{ItemCatalog, ItemKind, MAX_PLAYERS, WeaponKind};

pub const CROSSHAIR_WIDGET: &str = "hud_crosshair";
pub const HIT_WIDGET: &str = "hud_hit";
pub const HEALTH_WIDGET: &str = "hud_health";
pub const RESPAWN_WIDGET: &str = "hud_respawn";
pub const CLOCK_WIDGET: &str = "hud_match_clock";
pub const RESULT_WIDGET: &str = "hud_match_banner";
pub const SCORE_HEAD_WIDGET: &str = "hud_match_score_head";

/// The ammo line of one weapon.
pub fn ammo_widget(kind: WeaponKind) -> &'static str {
    match kind {
        WeaponKind::Rifle => "hud_ammo_rifle",
        WeaponKind::Launcher => "hud_ammo_launcher",
    }
}

/// The ids of the numbered lines, made once at startup.
#[derive(Resource, Debug)]
pub struct HudLines {
    /// `hud_match_score_row0`..: one per possible player.
    pub score_rows: Vec<&'static str>,
    /// `hud_kill0`..: one per line of the kill feed.
    pub kills: Vec<&'static str>,
    /// `hud_item0`..: one per item of the catalog that is used from the inventory.
    pub items: Vec<&'static str>,
}

/// Registers every widget, hidden.
pub(crate) fn register_widgets(app: &mut App, config: &CombatHudConfig) {
    let numbered = |prefix: &str, count: usize| -> Vec<&'static str> {
        (0..count)
            // Ids live as long as the app, which registers them once.
            .map(|line| &*Box::leak(format!("{prefix}{line}").into_boxed_str()))
            .collect()
    };
    let usable_items = app
        .world()
        .resource::<ItemCatalog>()
        .0
        .iter()
        .filter(|item| matches!(item.kind, ItemKind::Heal(_)))
        .count();
    let lines = HudLines {
        items: numbered("hud_item", usable_items),
        score_rows: numbered("hud_match_score_row", MAX_PLAYERS),
        kills: numbered("hud_kill", config.kill_feed_lines as usize),
    };
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
    });
    let size = config.font_size;
    for (id, anchor) in [
        (HIT_WIDGET, HudAnchor::Center),
        (HEALTH_WIDGET, HudAnchor::BottomLeft),
        (RESPAWN_WIDGET, HudAnchor::Center),
        (CLOCK_WIDGET, HudAnchor::TopCenter),
        (RESULT_WIDGET, HudAnchor::TopCenter),
    ] {
        app.register_hud_widget(text(id, anchor, size));
    }
    // Above the health, which sorts first at the same corner.
    for id in &lines.items {
        app.register_hud_widget(text(id, HudAnchor::BottomLeft, size));
    }
    for kind in WeaponKind::ALL {
        app.register_hud_widget(text(ammo_widget(kind), HudAnchor::BottomRight, size));
    }
    let small = config.scoreboard_font_size;
    for id in std::iter::once(SCORE_HEAD_WIDGET).chain(lines.score_rows.iter().copied()) {
        app.register_hud_widget(text(id, HudAnchor::TopCenter, small));
    }
    for id in &lines.kills {
        app.register_hud_widget(text(id, HudAnchor::TopRight, small));
    }
    app.insert_resource(lines);
}
