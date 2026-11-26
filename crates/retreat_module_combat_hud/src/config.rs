//! The module's tuning numbers.

use bevy::prelude::*;

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CombatHudConfig {
    /// How long the hit marker stays.
    pub hit_marker_seconds: f32,
    pub hit_marker_text: String,
    pub font_size: f32,
    /// Width of the crosshair in pixels.
    pub crosshair_size: f32,
    /// Text size of the scoreboard (up to a line per player, so smaller than the rest).
    pub scoreboard_font_size: f32,
    /// How long a death stays in the kill feed.
    pub kill_feed_seconds: f32,
    /// Most deaths in the kill feed at once.
    pub kill_feed_lines: u32,
    /// Behind every text, so it reads over a bright scene. Lines of one corner share one box.
    pub panel: Color,
}

impl Default for CombatHudConfig {
    fn default() -> Self {
        Self {
            hit_marker_seconds: 0.2,
            hit_marker_text: "HIT".to_string(),
            font_size: 28.0,
            crosshair_size: 24.0,
            scoreboard_font_size: 22.0,
            kill_feed_seconds: 4.0,
            kill_feed_lines: 4,
            panel: Color::srgba(0.0, 0.0, 0.0, 0.85),
        }
    }
}

impl CombatHudConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("hit_marker_seconds", self.hit_marker_seconds),
            ("font_size", self.font_size),
            ("crosshair_size", self.crosshair_size),
            ("scoreboard_font_size", self.scoreboard_font_size),
            ("kill_feed_seconds", self.kill_feed_seconds),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("{field} must be more than 0, got {value}"));
            }
        }
        Ok(())
    }
}
