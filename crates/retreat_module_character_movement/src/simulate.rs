//! Moving every character: ground detection, then the movement rule.

use crate::config::MovementConfig;
use crate::motion::{MovementStep, step_movement};
use crate::state::MovementState;
use bevy::prelude::*;
use retreat_engine_api::{
    PhysicsShape, PhysicsVelocity, PhysicsVelocitySet, RetiredEntity, ShapeCast, cast_shape,
};
use retreat_gameplay_api::{
    CHARACTER_CAPSULE_HALF_HEIGHT, CHARACTER_RADIUS, Health, LookAngles, PlayerId,
};

/// The ground probe is a ball a little narrower than the capsule, so walls are not ground.
const PROBE_RADIUS: f32 = CHARACTER_RADIUS * 0.9;
/// The probe's travel when the character rests on the ground.
const RESTING_DISTANCE: f32 = CHARACTER_RADIUS - PROBE_RADIUS;
/// How far above its resting height a character still counts as standing.
const GROUND_REACH: f32 = 0.05;

pub(crate) fn move_characters(world: &mut World) {
    let tick = world.resource::<Time<Fixed>>().timestep();
    let config = world.resource::<MovementConfig>().clone();
    let mut characters = world.query_filtered::<(
        Entity,
        &PlayerId,
        &MovementState,
        &LookAngles,
        &Transform,
        &PhysicsVelocity,
        Option<&Health>,
    ), Without<RetiredEntity>>();
    let mut found: Vec<_> = characters
        .iter(world)
        .map(
            |(entity, player, state, look, transform, velocity, health)| {
                (
                    *player,
                    entity,
                    *state,
                    look.yaw,
                    transform.translation,
                    velocity.linear,
                    health.is_none_or(Health::is_alive),
                )
            },
        )
        .collect();
    // By player id: query order can change after a rewind.
    found.sort_by_key(|(player, ..)| *player);
    for (_, entity, mut state, yaw, center, velocity, can_act) in found {
        let probe = cast_shape(
            world,
            &ShapeCast {
                shape: PhysicsShape::Ball {
                    radius: PROBE_RADIUS,
                },
                origin: center - Vec3::Y * CHARACTER_CAPSULE_HALF_HEIGHT,
                rotation: Quat::IDENTITY,
                direction: Vec3::NEG_Y,
                max_distance: RESTING_DISTANCE + GROUND_REACH,
                exclude: vec![entity],
            },
        );
        let step = MovementStep {
            velocity,
            ground_gap: probe.map(|hit| hit.distance - RESTING_DISTANCE),
            yaw,
            can_act,
            tick,
        };
        let next = step_movement(&mut state, step, &config);
        world
            .entity_mut(entity)
            .insert((state, PhysicsVelocitySet(next)));
    }
}
