//! A character's rolled-back and replicated movement state.

use crate::config::MovementConfig;
use bevy::prelude::*;
use retreat_engine_api::{
    CharacterStepping, NetState, PhysicsBody, PhysicsBodyKind, RetiredEntity,
};
use retreat_gameplay_api::{LookAngles, PlayerId};
use serde::{Deserialize, Serialize};

/// What a player wants this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// -1 left to 1 right.
    pub strafe: f32,
    /// -1 back to 1 forward.
    pub advance: f32,
    pub sprint: bool,
    pub jump: bool,
    pub dash: bool,
}

/// On every controllable character.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MovementState {
    /// The latest input; it stays until the next one arrives.
    pub input: PlayerInput,
    /// Jump and dash act when the button goes down, so the previous tick's buttons are kept.
    pub jump_was_down: bool,
    pub dash_was_down: bool,
    /// What the last tick's movement rule saw.
    pub grounded: bool,
    pub dash_ticks_left: u32,
    pub dash_cooldown_ticks: u32,
    pub dash_direction: Vec2,
    /// One dash per time in the air.
    pub air_dash_used: bool,
}

impl NetState for MovementState {
    const ID: &'static str = "movement.state";
}

type NewCharacters = (
    With<PlayerId>,
    Without<MovementState>,
    Without<RetiredEntity>,
);

/// Gives every player that has a character body its movement state and the engine's step handling.
pub(crate) fn attach_state(
    players: Query<(Entity, &PhysicsBody, Has<LookAngles>), NewCharacters>,
    config: Res<MovementConfig>,
    mut commands: Commands,
) {
    for (entity, body, has_look) in &players {
        if body.kind != PhysicsBodyKind::Character {
            continue;
        }
        commands.entity(entity).insert((
            MovementState::default(),
            CharacterStepping {
                max_step_height: config.step_height,
                max_slope_degrees: config.max_slope_degrees,
            },
        ));
        if !has_look {
            commands.entity(entity).insert(LookAngles::default());
        }
    }
}
