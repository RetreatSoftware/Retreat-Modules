//! Giving every player its looks.

use crate::config::CharacterVisualsConfig;
use crate::locomotion::{Locomotion, clips_for, facing_of, locomotion_of};
use bevy::prelude::*;
use retreat_engine_api::{
    CharacterGround, CharacterVisual, EntityColor, EntityColorFlat, HideVisuals, NetLocalClient,
    PhysicsVelocity, RemoteInterpolation, RenderSmoothing, RetiredEntity,
};
use retreat_gameplay_api::{
    CharacterSize, Health, LookAngles, PlayerId, SoloPlayer, local_player_id,
};

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
        Option<&'static Health>,
        (
            Has<RenderSmoothing>,
            Has<RemoteInterpolation>,
            Has<EntityColorFlat>,
            Has<HideVisuals>,
        ),
    ),
    (With<Name>, Without<RetiredEntity>),
>;

pub(crate) fn dress_players(
    players: Players,
    config: Res<CharacterVisualsConfig>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    size: Res<CharacterSize>,
    mut commands: Commands,
) {
    let local = local_player_id(*client, *solo);
    for (entity, player, velocity, ground, look, motion, visual, color, health, marks) in &players {
        let (smoothed, remote, flat, hidden) = marks;
        let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.linear);
        let before = motion.map_or_else(|| look.map_or(0.0, |look| look.yaw), |m| m.facing);
        let other = local.is_some_and(|local| local != *player);
        let now = VisualMotion {
            doing: locomotion_of(velocity, ground.map(|g| g.grounded), &config),
            // Another player turns with its drawn movement (`remote_facing`).
            facing: if other {
                before
            } else {
                facing_of(velocity, before, &config)
            },
        };
        let wanted = visual_for(&config, *size, now, velocity);
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
        if config.flat_colors && !flat {
            entity.insert(EntityColorFlat);
        }
        // The dead are not drawn.
        let dead = health.is_some_and(|health| !health.is_alive());
        if dead && !hidden {
            entity.insert(HideVisuals);
        } else if !dead && hidden {
            entity.remove::<HideVisuals>();
        }
        // A machine with a player draws every character between ticks, and the others from the
        // server's past states: what a lag-compensating weapon judges shots against.
        if local.is_some() && !smoothed {
            entity.insert(config.smoothing);
        }
        if other && !remote {
            entity.insert(RemoteInterpolation);
        } else if !other && remote {
            entity.remove::<RemoteInterpolation>();
        }
    }
}

fn visual_for(
    config: &CharacterVisualsConfig,
    size: CharacterSize,
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
            offset: Vec3::NEG_Y * (size.height / 2.0),
            yaw: motion.facing + model.yaw,
            clips: clips_for(model, motion.doing, velocity),
        },
    }
}
