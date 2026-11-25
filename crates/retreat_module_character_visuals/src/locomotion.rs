//! What a character is doing, which way it faces and what it plays: pure rules.

use crate::config::{CharacterModel, CharacterVisualsConfig};
use bevy::math::ops;
use bevy::prelude::*;
use retreat_engine_api::{ClipPlayback, MAX_CLIP_SPEED};

/// Playback speeds are rounded to this step, so tiny speed changes do not restate the animation.
const RATE_STEP: f32 = 0.1;

/// What a character is doing, as far as its looks go.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Locomotion {
    #[default]
    Idle,
    Walk,
    Run,
    Air,
}

impl Locomotion {
    /// The word scripts use.
    pub fn name(self) -> &'static str {
        match self {
            Locomotion::Idle => "idle",
            Locomotion::Walk => "walk",
            Locomotion::Run => "run",
            Locomotion::Air => "air",
        }
    }
}

/// `grounded` is the engine's readout, or `None` for a character without one.
pub fn locomotion_of(
    velocity: Vec3,
    grounded: Option<bool>,
    config: &CharacterVisualsConfig,
) -> Locomotion {
    let airborne = match grounded {
        Some(grounded) => !grounded,
        None => velocity.y.abs() > config.air_vertical_speed,
    };
    let speed = velocity.xz().length();
    if airborne {
        Locomotion::Air
    } else if speed < config.idle_below {
        Locomotion::Idle
    } else if speed < config.run_from {
        Locomotion::Walk
    } else {
        Locomotion::Run
    }
}

/// The yaw (0 looks along -Z) of the movement, or `previous` for a character that barely moves
/// along the ground.
pub fn facing_of(velocity: Vec3, previous: f32, config: &CharacterVisualsConfig) -> f32 {
    if velocity.xz().length() < config.idle_below {
        previous
    } else {
        ops::atan2(-velocity.x, -velocity.z)
    }
}

/// The model's animation for what the character does at `velocity`.
pub fn clips_for(model: &CharacterModel, doing: Locomotion, velocity: Vec3) -> Vec<ClipPlayback> {
    let clip = match doing {
        Locomotion::Idle => &model.idle,
        Locomotion::Walk => &model.walk,
        Locomotion::Run => &model.run,
        Locomotion::Air => &model.air,
    };
    let pace = clip
        .matched_speed
        .map_or(1.0, |matched| velocity.xz().length() / matched);
    let rate = ((clip.rate * pace) / RATE_STEP).round() * RATE_STEP;
    vec![ClipPlayback {
        speed: rate.clamp(0.0, MAX_CLIP_SPEED),
        ..ClipPlayback::looping(clip.name.clone())
    }]
}
