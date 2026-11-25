//! Moving every character with the movement rule.

use crate::config::MovementConfig;
use crate::motion::{MovementStep, step_movement};
use crate::state::MovementState;
use bevy::prelude::*;
use retreat_engine_api::{CharacterGround, PhysicsVelocity, PhysicsVelocitySet, RetiredEntity};
use retreat_gameplay_api::{Health, LookAngles, PlayerId};

type Characters<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        &'static mut MovementState,
        &'static LookAngles,
        &'static PhysicsVelocity,
        Option<&'static CharacterGround>,
        Option<&'static Health>,
    ),
    Without<RetiredEntity>,
>;

pub(crate) fn move_characters(
    mut characters: Characters,
    config: Res<MovementConfig>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    for (entity, _, mut state, look, velocity, ground, health) in &mut characters {
        let step = MovementStep {
            velocity: velocity.linear,
            grounded: ground.is_some_and(|ground| ground.grounded),
            yaw: look.yaw,
            can_act: health.is_none_or(Health::is_alive),
            tick: time.timestep(),
        };
        let mut next = *state;
        let wanted = step_movement(&mut next, step, &config);
        state.set_if_neq(next);
        commands.entity(entity).insert(PhysicsVelocitySet(wanted));
    }
}
