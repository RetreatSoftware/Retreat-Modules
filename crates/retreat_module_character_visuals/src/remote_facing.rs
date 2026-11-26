//! Which way **another** player faces. It is drawn from the server's past (`RemoteInterpolation`),
//! so its facing comes from how that picture moves, not from the predicted present velocity: with two
//! time bases the character turned away from its path at the end of a turn.

use crate::config::CharacterVisualsConfig;
use crate::dress::VisualMotion;
use crate::locomotion::facing_of;
use bevy::prelude::*;
use retreat_engine_api::{CharacterVisual, DrawnPose, RemoteInterpolation, RetiredEntity};

/// Where the character was drawn last frame.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct DrawnBefore(Vec3);

/// The facing of a character drawn at `from` and, `seconds` later, at `to`. A longer way than
/// `config.smoothing.snap_distance` is a teleport and keeps `previous`, like a character that
/// barely moves.
pub fn facing_of_drawn_movement(
    from: Vec3,
    to: Vec3,
    seconds: f32,
    previous: f32,
    config: &CharacterVisualsConfig,
) -> f32 {
    if seconds <= 0.0 || from.distance(to) > config.smoothing.snap_distance {
        previous
    } else {
        facing_of((to - from) / seconds, previous, config)
    }
}

type Remotes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static DrawnPose,
        Option<&'static DrawnBefore>,
        &'static mut VisualMotion,
        &'static mut CharacterVisual,
    ),
    (With<RemoteInterpolation>, Without<RetiredEntity>),
>;

pub(crate) fn face_remote_players(
    mut remotes: Remotes,
    config: Res<CharacterVisualsConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let model_yaw = config.model.as_ref().map_or(0.0, |model| model.yaw);
    for (entity, drawn, before, mut motion, mut visual) in &mut remotes {
        if let Some(before) = before {
            let facing = facing_of_drawn_movement(
                before.0,
                drawn.translation,
                time.delta_secs(),
                motion.facing,
                &config,
            );
            if facing != motion.facing {
                motion.facing = facing;
                visual.yaw = facing + model_yaw;
            }
        }
        commands
            .entity(entity)
            .insert(DrawnBefore(drawn.translation));
    }
}
