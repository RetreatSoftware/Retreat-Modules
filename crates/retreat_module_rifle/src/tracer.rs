//! What a shot looks and sounds like: a flash at the muzzle, a streak of glowing dots to where it
//! went, and the shot from the shooter. All are the engine's one-shot events with ids made from the
//! shooter and its shot number, so a replayed or corrected tick never repeats a shot.

use crate::config::RifleConfig;
use crate::state::{RifleShot, RifleState};
use bevy::prelude::*;
use retreat_engine_api::{
    EffectTime, NetLocalClient, RetiredEntity, SoundEvents, VisualEffect, VisualEffects,
};
use retreat_gameplay_api::{
    PlayerId, SoloPlayer, WeaponKind, Weapons, event_time, local_player_id,
};
use retreat_weapon_kit::play_reload;
use std::collections::BTreeMap;

/// The muzzle is this far from the eyes along the shot.
const MUZZLE_DISTANCE: f32 = 0.6;
/// Metres between two dots, and the most dots of one shot (long shots space them further).
const DOT_SPACING: f32 = 0.7;
const MOST_DOTS: usize = 32;
const TRACER_COLOR: Color = Color::srgb(1.0, 0.85, 0.35);
const DOT_RADIUS: f32 = 0.065;
const DOT_TICKS: u32 = 10;
const FLASH_RADIUS: f32 = 0.2;
const FLASH_TICKS: u32 = 5;

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

/// Where the shot's looks and sound go.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ShotOutput<'w> {
    config: Res<'w, RifleConfig>,
    effects: ResMut<'w, VisualEffects>,
    sounds: ResMut<'w, SoundEvents>,
}

impl ShotOutput<'_> {
    pub(crate) fn play(&mut self, player: PlayerId, shot: RifleShot, time: EffectTime) {
        let id = format!("rifle/{}/{}", player.0, shot.count);
        if let Some(cue) = &self.config.shot_sound {
            self.sounds
                .play(cue.event(id.clone(), Some(shot.from), time));
        }
        let Some(direction) = (shot.to - shot.from).try_normalize() else {
            return;
        };
        let muzzle = shot.from + direction * MUZZLE_DISTANCE.min(shot.from.distance(shot.to));
        let effect = |id, position, radius, ticks| VisualEffect {
            id,
            position,
            radius,
            color: TRACER_COLOR,
            glow: 3.0,
            ticks,
            time,
        };
        self.effects.play(effect(
            format!("{id}/flash"),
            muzzle,
            FLASH_RADIUS,
            FLASH_TICKS,
        ));
        for (dot, position) in tracer_points(muzzle, shot.to).into_iter().enumerate() {
            self.effects.play(effect(
                format!("{id}/{dot}"),
                position,
                DOT_RADIUS,
                DOT_TICKS,
            ));
        }
    }
}

/// A client does not run the others' fire commands: it sees their shot count rise and shows the
/// shot when its view of them gets there. The first count seen of a player shows nothing (it may be
/// old).
pub(crate) fn show_other_shots(
    players: Query<(&PlayerId, &RifleShot), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    mut seen: Local<BTreeMap<PlayerId, u32>>,
    mut output: ShotOutput,
) {
    let Some(local) = client.0 else {
        return;
    };
    for (player, shot) in &players {
        let before = seen.insert(*player, shot.count);
        if player.0 != local && before.is_some_and(|before| shot.count > before) {
            output.play(*player, *shot, event_time(*client, *player));
        }
    }
}

type Reloaders<'w, 's> = Query<
    'w,
    's,
    (&'static PlayerId, &'static Weapons, &'static RifleShot),
    (With<RifleState>, Without<RetiredEntity>),
>;

/// This machine's player hears its rifle reload.
pub(crate) fn play_own_reload(
    players: Reloaders,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    config: Res<RifleConfig>,
    mut sounds: ResMut<SoundEvents>,
) {
    let local = local_player_id(*client, *solo);
    for (player, weapons, shot) in &players {
        if Some(*player) == local
            && let Some(weapon) = weapons.of(WeaponKind::Rifle)
        {
            let cue = config.reload_sound.as_ref();
            play_reload(&mut sounds, cue, *player, weapon, shot.count);
        }
    }
}
