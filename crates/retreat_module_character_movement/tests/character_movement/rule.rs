//! `step_movement`.

use bevy::prelude::*;
use retreat_module_character_movement::{
    MovementConfig, MovementState, MovementStep, PlayerInput, step_movement,
};
use std::time::Duration;

const TICK: Duration = Duration::from_micros(15_625);

/// A character on flat ground or in the air, with the body doing exactly what it is told.
struct Body {
    state: MovementState,
    velocity: Vec3,
    height: f32,
}

impl Body {
    fn standing() -> Self {
        Self {
            state: MovementState::default(),
            velocity: Vec3::ZERO,
            height: 0.0,
        }
    }

    fn tick(&mut self, input: PlayerInput) -> Vec3 {
        self.tick_as(input, true)
    }

    fn tick_as(&mut self, input: PlayerInput, can_act: bool) -> Vec3 {
        self.state.input = input;
        let step = MovementStep {
            velocity: self.velocity,
            grounded: self.height <= 0.0 && self.velocity.y <= 0.0,
            yaw: 0.0,
            can_act,
            tick: TICK,
        };
        self.velocity = step_movement(&mut self.state, step, &MovementConfig::default());
        self.height = (self.height + self.velocity.y * TICK.as_secs_f32()).max(0.0);
        self.velocity
    }

    fn run(&mut self, input: PlayerInput, ticks: u32) -> Vec3 {
        (0..ticks).fold(Vec3::ZERO, |_, _| self.tick(input))
    }
}

fn forward() -> PlayerInput {
    PlayerInput {
        advance: 1.0,
        ..default()
    }
}

#[test]
fn walking_and_sprinting_reach_their_speeds_and_stopping_stops() {
    let mut body = Body::standing();
    let walk = body.run(forward(), 64);
    assert!(
        walk.abs_diff_eq(Vec3::new(0.0, 0.0, -6.0), 1.0e-3),
        "{walk}"
    );
    let sprint = body.run(
        PlayerInput {
            sprint: true,
            ..forward()
        },
        64,
    );
    assert!((sprint.z + 9.0).abs() < 1.0e-3, "{sprint}");
    assert_eq!(body.run(PlayerInput::default(), 64), Vec3::ZERO);
}

#[test]
fn diagonals_are_not_faster_and_right_is_plus_x() {
    let mut body = Body::standing();
    let diagonal = body.run(
        PlayerInput {
            strafe: 1.0,
            ..forward()
        },
        64,
    );
    assert!((diagonal.xz().length() - 6.0).abs() < 1.0e-3);
    assert!(diagonal.x > 0.0 && diagonal.z < 0.0);
}

#[test]
fn a_jump_reaches_about_its_height_and_needs_a_new_press() {
    let mut body = Body::standing();
    let jump = PlayerInput {
        jump: true,
        ..default()
    };
    let mut apex: f32 = 0.0;
    for _ in 0..128 {
        body.tick(jump);
        apex = apex.max(body.height);
    }
    assert!((apex - 1.5).abs() < 0.1, "apex {apex}");
    // Held through the landing: still on the ground.
    assert!(body.state.grounded);
    assert_eq!(body.height, 0.0);
    body.tick(PlayerInput::default());
    assert!(body.tick(jump).y > 7.0);
    assert!(!body.state.grounded);
}

#[test]
fn air_control_adds_little_speed() {
    let mut body = Body::standing();
    body.height = 10.0;
    let drift = body.run(forward(), 20);
    assert!((drift.z + 1.0).abs() < 1.0e-3, "{drift}");
    assert!(drift.y < -5.0);
    body.height = 1000.0;
    assert_eq!(body.run(forward(), 200).y, -25.0, "falling has a top speed");
}

#[test]
fn a_dash_covers_its_distance_then_waits_for_the_cooldown() {
    let mut body = Body::standing();
    let dash = PlayerInput {
        dash: true,
        ..default()
    };
    let mut distance = 0.0;
    for _ in 0..10 {
        distance += -body.tick(dash).z * TICK.as_secs_f32();
    }
    // 9 ticks at 18 m/s (the 10th tick of 0.15 s ends it at the walking speed, here zero).
    assert!((distance - 18.0 * 9.0 / 64.0).abs() < 1.0e-3, "{distance}");
    body.run(PlayerInput::default(), 5);
    assert_eq!(body.tick(dash), Vec3::ZERO, "cooling down");
    body.run(PlayerInput::default(), 192);
    assert!(body.tick(dash).z < -17.0);
}

#[test]
fn one_dash_per_time_in_the_air() {
    let mut body = Body::standing();
    body.height = 1000.0;
    let dash = PlayerInput {
        dash: true,
        ..default()
    };
    assert!(body.tick(dash).z < -17.0);
    body.run(PlayerInput::default(), 250);
    assert!(body.tick(dash).z > -17.0, "already used in this fall");
    body.height = 0.0;
    body.run(PlayerInput::default(), 200);
    assert!(body.tick(dash).z < -17.0);
}

#[test]
fn the_dead_cannot_steer_jump_or_dash() {
    let mut body = Body::standing();
    let everything = PlayerInput {
        advance: 1.0,
        sprint: true,
        jump: true,
        dash: true,
        ..default()
    };
    assert_eq!(body.tick_as(everything, false), Vec3::ZERO);
}
