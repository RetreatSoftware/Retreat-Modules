//! The movement rule: one tick of velocity, without a Bevy app or physics.

use crate::config::MovementConfig;
use crate::state::MovementState;
use bevy::math::ops;
use bevy::prelude::*;
use retreat_gameplay_api::{horizontal_axes, ticks_from_seconds};
use std::time::Duration;

/// Ticks after a jump in which the ground is ignored.
const JUMP_GROUND_IGNORE_TICKS: u32 = 8;

/// What one tick of movement starts from.
#[derive(Debug, Clone, Copy)]
pub struct MovementStep {
    /// The body's velocity as physics left it.
    pub velocity: Vec3,
    /// How far above its resting height the character is, when the ground is within reach.
    pub ground_gap: Option<f32>,
    pub yaw: f32,
    /// False for the dead: they fall and slide to a stop but do not steer.
    pub can_act: bool,
    pub tick: Duration,
}

/// Advances `state` by one tick and returns the velocity the body should get.
pub fn step_movement(
    state: &mut MovementState,
    step: MovementStep,
    config: &MovementConfig,
) -> Vec3 {
    let dt = step.tick.as_secs_f32();
    let input = if step.can_act {
        state.input
    } else {
        Default::default()
    };
    let jump_pressed = input.jump && !state.jump_was_down;
    let dash_pressed = input.dash && !state.dash_was_down;
    state.jump_was_down = input.jump;
    state.dash_was_down = input.dash;

    state.ticks_ignoring_ground = state.ticks_ignoring_ground.saturating_sub(1);
    state.dash_cooldown_ticks = state.dash_cooldown_ticks.saturating_sub(1);
    state.grounded = step.ground_gap.is_some() && state.ticks_ignoring_ground == 0;
    if state.grounded {
        state.air_dash_used = false;
    }

    let (forward, right) = horizontal_axes(step.yaw);
    let wish = (right * input.strafe + forward * input.advance).clamp_length_max(1.0);
    let wish_speed = wish.length()
        * if input.sprint {
            config.sprint_speed
        } else {
            config.walk_speed
        };
    let wish_direction = wish.normalize_or_zero().xz();

    if dash_pressed && state.dash_cooldown_ticks == 0 && (state.grounded || !state.air_dash_used) {
        // Towards the movement keys, or where the character faces.
        state.dash_direction = if wish_direction == Vec2::ZERO {
            forward.xz()
        } else {
            wish_direction
        };
        state.dash_ticks_left = ticks_from_seconds(config.dash_seconds, step.tick);
        state.dash_cooldown_ticks = ticks_from_seconds(config.dash_cooldown_seconds, step.tick);
        state.air_dash_used = !state.grounded;
    }
    if state.dash_ticks_left > 0 && step.can_act {
        state.dash_ticks_left -= 1;
        // After the last dash tick the character continues at its normal speed.
        let speed = if state.dash_ticks_left > 0 {
            config.dash_speed
        } else {
            wish_speed
        };
        let flat = state.dash_direction * speed;
        return Vec3::new(flat.x, 0.0, flat.y);
    }
    state.dash_ticks_left = 0;

    let mut flat = step.velocity.xz();
    let vertical;
    if state.grounded && jump_pressed {
        // The speed that reaches `jump_height` against gravity.
        vertical = ops::sqrt(2.0 * config.gravity * config.jump_height);
        state.grounded = false;
        state.ticks_ignoring_ground = JUMP_GROUND_IGNORE_TICKS;
        flat = accelerate(
            flat,
            wish_direction,
            wish_speed,
            wish_speed,
            config.air_acceleration,
            dt,
        );
    } else if state.grounded {
        // Held onto the ground, so walking down a step does not become falling.
        vertical = -(step.ground_gap.unwrap_or(0.0).max(0.0) / dt).min(config.max_fall_speed);
        flat = apply_friction(flat, config, dt);
        flat = accelerate(
            flat,
            wish_direction,
            wish_speed,
            wish_speed,
            config.ground_acceleration,
            dt,
        );
    } else {
        vertical = (step.velocity.y - config.gravity * dt).max(-config.max_fall_speed);
        let capped = wish_speed.min(config.air_wish_speed);
        flat = accelerate(
            flat,
            wish_direction,
            capped,
            wish_speed,
            config.air_acceleration,
            dt,
        );
    }
    Vec3::new(flat.x, vertical, flat.y)
}

fn apply_friction(velocity: Vec2, config: &MovementConfig, dt: f32) -> Vec2 {
    let speed = velocity.length();
    if speed <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let drop = speed.max(config.stop_speed) * config.ground_friction * dt;
    velocity * ((speed - drop).max(0.0) / speed)
}

/// Adds speed along `direction` until the speed in that direction is `target`; `strength` sets how
/// much one tick may add.
fn accelerate(
    velocity: Vec2,
    direction: Vec2,
    target: f32,
    strength: f32,
    acceleration: f32,
    dt: f32,
) -> Vec2 {
    let missing = target - velocity.dot(direction);
    if missing <= 0.0 {
        return velocity;
    }
    velocity + direction * (acceleration * strength * dt).min(missing)
}
