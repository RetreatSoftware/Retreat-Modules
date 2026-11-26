//! How the HUD's text looks. Pure.

use bevy::prelude::*;

/// How much white a player's color gets in text, so dark colors read on a dark panel.
const LIGHTEN: f32 = 0.4;

/// A player's color as text on the panel: mixed towards white.
pub fn readable(color: Color) -> Color {
    let color = color.to_srgba();
    let lighter = |channel: f32| channel + (1.0 - channel) * LIGHTEN;
    Color::srgb(
        lighter(color.red),
        lighter(color.green),
        lighter(color.blue),
    )
}
