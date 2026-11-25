//! Giving every player its looks.

use crate::config::CharacterVisualsConfig;
use crate::locomotion::{Locomotion, clips_for, facing_of, locomotion_of};
use bevy::prelude::*;
use retreat_engine_api::{
    CharacterGround, CharacterVisual, EntityColor, NetLocalClient, PhysicsVelocity,
    RenderSmoothing, RetiredEntity,
};
use retreat_gameplay_api::{CHARACTER_HEIGHT, LookAngles, PlayerId, SoloPlayer, local_player_id};

/// What this machine shows a player doing. Not shared state: every machine derives its own.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct VisualMotion {
    pub doing: Locomotion,
    /// Yaw the character is drawn with.
    pub facing: f32,
}

type Players<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        Option<&'static PhysicsVelocity>,
        Option<&'static CharacterGround>,
        Option<&'static LookAngles>,
        Option<&'static VisualMotion>,
        Option<&'static CharacterVisual>,
        Option<&'static EntityColor>,
        Has<RenderSmoothing>,
    ),
    (With<Name>, Without<RetiredEntity>),
>;

pub(crate) fn dress_players(
    players: Players,
    config: Res<CharacterVisualsConfig>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    mut commands: Commands,
) {
    let local = local_player_id(*client, *solo);
    for (entity, player, velocity, ground, look, motion, visual, color, smoothed) in &players {
        let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.linear);
        let before = motion.map_or_else(|| look.map_or(0.0, |look| look.yaw), |m| m.facing);
        let now = VisualMotion {
            doing: locomotion_of(velocity, ground.map(|g| g.grounded), &config),
            facing: facing_of(velocity, before, &config),
        };
        let wanted = visual_for(&config, now, velocity);
        let wanted_color = EntityColor(config.colors[player.0 as usize % config.colors.len()]);
        let mut entity = commands.entity(entity);
        if motion != Some(&now) {
            entity.insert(now);
        }
        if visual != Some(&wanted) {
            entity.insert(wanted);
        }
        if color != Some(&wanted_color) {
            entity.insert(wanted_color);
        }
        // Others are drawn between ticks; the own player follows the input (and the camera) exactly.
        let smooth = local.is_some_and(|local| local != *player);
        if smooth && !smoothed {
            entity.insert(config.smoothing);
        } else if !smooth && smoothed {
            entity.remove::<RenderSmoothing>();
        }
    }
}

fn visual_for(
    config: &CharacterVisualsConfig,
    motion: VisualMotion,
    velocity: Vec3,
) -> CharacterVisual {
    match &config.model {
        None => CharacterVisual {
            yaw: motion.facing,
            ..default()
        },
        Some(model) => CharacterVisual {
            model: Some(model.path.clone()),
            scale: model.scale,
            // The entity is the middle of the character; the model stands on its feet.
            offset: Vec3::NEG_Y * (CHARACTER_HEIGHT / 2.0),
            yaw: motion.facing + model.yaw,
            clips: clips_for(model, motion.doing, velocity),
        },
    }
}
