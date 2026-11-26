//! What characters look like. The numbers and names are data: a game sets them.

use bevy::prelude::*;
use retreat_engine_api::RenderSmoothing;

/// One animation clip of the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// The clip's name in the model file.
    pub name: String,
    /// Playback speed (1 is the file's own).
    pub rate: f32,
    /// For clips of moving: the ground speed (m/s) at which `rate` looks right; the playback then
    /// follows the character's speed. `None` plays at `rate` whatever the speed.
    pub matched_speed: Option<f32>,
}

/// An animated model and which clip shows what.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterModel {
    /// A glTF path in the app's assets.
    pub path: String,
    /// Uniform scale. The model's origin must be at its feet.
    pub scale: f32,
    /// Turn that points the model's front to -Z, radians.
    pub yaw: f32,
    pub idle: Clip,
    pub walk: Clip,
    pub run: Clip,
    pub air: Clip,
}

/// A game changes this by inserting the resource before adding the plugin. Each player's color is
/// the contract's `PlayerRoster`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CharacterVisualsConfig {
    /// `None` draws the engine's plain shape with its facing.
    pub model: Option<CharacterModel>,
    /// Replace the model's texture with the player's color instead of tinting it.
    pub flat_colors: bool,
    /// Ground speed below which a character stands (m/s).
    pub idle_below: f32,
    /// Ground speed from which a character runs (m/s).
    pub run_from: f32,
    /// Without the engine's ground readout, a vertical speed above this counts as airborne.
    pub air_vertical_speed: f32,
    /// How characters are drawn between ticks and corrections.
    pub smoothing: RenderSmoothing,
}

impl Default for CharacterVisualsConfig {
    fn default() -> Self {
        Self {
            model: None,
            flat_colors: false,
            idle_below: 0.5,
            run_from: 7.5,
            air_vertical_speed: 1.0,
            smoothing: RenderSmoothing::default(),
        }
    }
}

impl CharacterVisualsConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("idle_below", self.idle_below),
            ("run_from", self.run_from),
            ("air_vertical_speed", self.air_vertical_speed),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("{field} must be more than 0, got {value}"));
            }
        }
        let Some(model) = &self.model else {
            return Ok(());
        };
        if !(model.scale.is_finite() && model.scale > 0.0 && model.yaw.is_finite()) {
            return Err("model scale must be more than 0 and model yaw finite".to_string());
        }
        for clip in [&model.idle, &model.walk, &model.run, &model.air] {
            if clip
                .matched_speed
                .is_some_and(|speed| speed.is_nan() || speed <= 0.0)
            {
                return Err(format!(
                    "clip `{}`: matched_speed must be more than 0",
                    clip.name
                ));
            }
            if !(clip.rate.is_finite() && clip.rate >= 0.0) {
                return Err(format!("clip `{}`: rate must be 0 or more", clip.name));
            }
        }
        Ok(())
    }
}
