//! The rifle's rolled-back and replicated state, and the systems that advance it.

use crate::config::RifleConfig;
use crate::rules::{RifleTicks, try_reload};
use bevy::prelude::*;
use retreat_engine_api::{NetState, RetiredEntity};
use retreat_gameplay_api::{
    DamageApplied, DamageCause, Health, PlayerId, PlayerRespawned, WeaponStatus,
};
use serde::{Deserialize, Serialize};

/// What only the rifle needs to know; ammo and reload are in the contract's `WeaponStatus`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RifleState {
    /// Ticks until the next shot.
    pub cooldown_ticks: u32,
}

impl NetState for RifleState {
    const ID: &'static str = "rifle.state";
}

type Unarmed = (With<PlayerId>, Without<RifleState>, Without<RetiredEntity>);

/// Every player gets a loaded rifle.
pub(crate) fn attach_weapons(
    players: Query<Entity, Unarmed>,
    config: Res<RifleConfig>,
    mut commands: Commands,
) {
    for entity in &players {
        commands.entity(entity).insert((
            RifleState::default(),
            WeaponStatus {
                ammo: config.magazine,
                magazine: config.magazine,
                reload_ticks_left: 0,
                hits_landed: 0,
            },
        ));
    }
}

type Armed<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        &'static mut WeaponStatus,
        &'static mut RifleState,
        Option<&'static Health>,
    ),
    Without<RetiredEntity>,
>;

/// One tick of cooldown and reload; an empty magazine starts to reload by itself.
pub(crate) fn advance_timers(mut players: Armed, config: Res<RifleConfig>, time: Res<Time<Fixed>>) {
    let ticks = RifleTicks::of(&config, time.timestep());
    for (_, mut weapon, mut rifle, health) in &mut players {
        if rifle.cooldown_ticks > 0 {
            rifle.cooldown_ticks -= 1;
        }
        if weapon.reload_ticks_left > 0 {
            weapon.reload_ticks_left -= 1;
            if weapon.reload_ticks_left == 0 {
                weapon.ammo = weapon.magazine;
            }
        } else if weapon.ammo == 0 && health.is_none_or(Health::is_alive) {
            try_reload(&mut weapon, ticks);
        }
    }
}

/// Counts the shots that really hurt someone.
pub(crate) fn count_hits(mut damage: MessageReader<DamageApplied>, mut players: Armed) {
    for applied in damage.read() {
        if applied.cause != DamageCause::Hitscan || applied.amount == 0 {
            continue;
        }
        for (player, mut weapon, ..) in &mut players {
            if applied.attacker == Some(*player) && applied.target != *player {
                weapon.hits_landed += 1;
            }
        }
    }
}

/// A new life starts with a full magazine.
pub(crate) fn refill_respawned(mut respawned: MessageReader<PlayerRespawned>, mut players: Armed) {
    for message in respawned.read() {
        for (player, mut weapon, mut rifle, _) in &mut players {
            if *player == message.player {
                weapon.ammo = weapon.magazine;
                weapon.reload_ticks_left = 0;
                rifle.cooldown_ticks = 0;
            }
        }
    }
}
