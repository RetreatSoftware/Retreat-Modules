//! What a shot looks like: a flash at the muzzle and a short line of glowing dots to where it went.
//! Both are the engine's one-shot effects with ids made from the shooter and its shot number, so a
//! replayed or corrected tick never shows a shot twice.

use crate::state::RifleShot;
use bevy::prelude::*;
use retreat_engine_api::{EffectTime, NetLocalClient, RetiredEntity, VisualEffect, VisualEffects};
use retreat_gameplay_api::PlayerId;
use std::collections::BTreeMap;

/// The muzzle is this far from the eyes along the shot.
const MUZZLE_DISTANCE: f32 = 0.6;
/// Metres between two dots, and the most dots of one shot (long shots space them further).
const DOT_SPACING: f32 = 1.2;
const MOST_DOTS: usize = 20;
const TRACER_COLOR: Color = Color::srgb(1.0, 0.85, 0.35);
const DOT_RADIUS: f32 = 0.07;
const DOT_TICKS: u32 = 10;
const FLASH_RADIUS: f32 = 0.22;
const FLASH_TICKS: u32 = 6;

/// Where the dots of a tracer from `muzzle` to `end` are, nearest first: evenly spread, none at the
/// muzzle itself, the last one at the end. Empty for a shot of no length.
pub fn tracer_points(muzzle: Vec3, end: Vec3) -> Vec<Vec3> {
    let length = muzzle.distance(end);
    if !(length.is_finite() && length > 0.0) {
        return Vec::new();
    }
    let dots = ((length / DOT_SPACING).ceil() as usize).clamp(1, MOST_DOTS);
    (1..=dots)
        .map(|dot| muzzle.lerp(end, dot as f32 / dots as f32))
        .collect()
}

pub(crate) fn play_shot(
    effects: &mut VisualEffects,
    player: PlayerId,
    shot: RifleShot,
    time: EffectTime,
) {
    let Some(direction) = (shot.to - shot.from).try_normalize() else {
        return;
    };
    let muzzle = shot.from + direction * MUZZLE_DISTANCE.min(shot.from.distance(shot.to));
    let id = format!("rifle/{}/{}", player.0, shot.count);
    let effect = |id, position, radius, ticks| VisualEffect {
        id,
        position,
        radius,
        color: TRACER_COLOR,
        glow: 3.0,
        ticks,
        time,
    };
    effects.play(effect(
        format!("{id}/flash"),
        muzzle,
        FLASH_RADIUS,
        FLASH_TICKS,
    ));
    for (dot, position) in tracer_points(muzzle, shot.to).into_iter().enumerate() {
        effects.play(effect(
            format!("{id}/{dot}"),
            position,
            DOT_RADIUS,
            DOT_TICKS,
        ));
    }
}

/// A client does not run the others' fire commands: it sees their shot count rise and shows the
/// shot when its view of them gets there. The first count seen of a player shows nothing (it may be
/// old).
pub(crate) fn show_other_shots(
    players: Query<(&PlayerId, &RifleShot), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    mut seen: Local<BTreeMap<PlayerId, u32>>,
    mut effects: ResMut<VisualEffects>,
) {
    let Some(local) = client.0 else {
        return;
    };
    for (player, shot) in &players {
        let before = seen.insert(*player, shot.count);
        if player.0 != local && before.is_some_and(|before| shot.count > before) {
            play_shot(&mut effects, *player, *shot, EffectTime::RemoteView);
        }
    }
}
