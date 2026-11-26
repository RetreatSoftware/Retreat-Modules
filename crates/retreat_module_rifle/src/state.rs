//! The rifle's rolled-back and replicated state, and the systems that advance it.

use crate::config::RifleConfig;
use bevy::prelude::*;
use retreat_engine_api::{NetState, RetiredEntity};
use retreat_gameplay_api::{
    AmmoSource, DamageApplied, DamageCause, Health, Inventory, ItemCatalog, PlayerId,
    PlayerRespawned, WeaponKind, Weapons,
};
use retreat_weapon_kit::{WeaponTicks, advance_weapon, equip, loaded};
use serde::{Deserialize, Serialize};

/// What only the rifle needs to know; ammo and reload are in the contract's `Weapons`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RifleState {
    /// Ticks until the next shot.
    pub cooldown_ticks: u32,
}

impl NetState for RifleState {
    const ID: &'static str = "rifle.state";
}

/// The player's last shot, so that every machine can show it.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct RifleShot {
    /// Shots fired since the player appeared; a rise is a new shot.
    pub count: u32,
    /// The eyes it left from.
    pub from: Vec3,
    /// What it hit, or the end of its range.
    pub to: Vec3,
}

impl NetState for RifleShot {
    const ID: &'static str = "rifle.shot";
}

pub(crate) fn ticks_of(config: &RifleConfig, time: &Time<Fixed>) -> WeaponTicks {
    WeaponTicks::of(
        config.fire_interval_seconds,
        config.reload_seconds,
        time.timestep(),
    )
}

type Unarmed = (With<PlayerId>, Without<RifleState>, Without<RetiredEntity>);

/// Every player gets a loaded rifle.
pub(crate) fn attach_rifles(
    players: Query<Entity, Unarmed>,
    config: Res<RifleConfig>,
    mut commands: Commands,
) {
    for entity in &players {
        commands
            .entity(entity)
            .insert((RifleState::default(), RifleShot::default()));
        equip(
            &mut commands,
            entity,
            loaded(WeaponKind::Rifle, config.magazine),
        );
    }
}

/// What a tick of the weapon reads and changes.
type WeaponParts<State> = (
    &'static mut Weapons,
    &'static mut State,
    Option<&'static Health>,
    Option<&'static mut Inventory>,
);

/// One tick of cooldown and reload.
pub(crate) fn advance_timers(
    mut players: Query<WeaponParts<RifleState>, Without<RetiredEntity>>,
    catalog: Res<ItemCatalog>,
    config: Res<RifleConfig>,
    time: Res<Time<Fixed>>,
) {
    let ticks = ticks_of(&config, &time);
    for (mut weapons, mut rifle, health, inventory) in &mut players {
        let Some(mut weapon) = weapons.of(WeaponKind::Rifle).copied() else {
            continue;
        };
        let mut next = *rifle;
        let alive = health.is_none_or(Health::is_alive);
        let mut carried = inventory.as_deref().cloned();
        let mut source = AmmoSource {
            inventory: carried.as_mut(),
            catalog: &catalog,
            weapon: WeaponKind::Rifle,
        };
        advance_weapon(
            &mut weapon,
            &mut next.cooldown_ticks,
            ticks,
            alive,
            &mut source,
        );
        if let (Some(mut inventory), Some(carried)) = (inventory, carried) {
            inventory.set_if_neq(carried);
        }
        rifle.set_if_neq(next);
        if weapons.of(WeaponKind::Rifle) != Some(&weapon) {
            weapons.put(weapon);
        }
    }
}

/// Counts the shots that really hurt someone.
pub(crate) fn count_hits(
    mut damage: MessageReader<DamageApplied>,
    mut players: Query<(&PlayerId, &mut Weapons), Without<RetiredEntity>>,
) {
    retreat_weapon_kit::count_hits(&mut damage, &mut players, WeaponKind::Rifle, |applied| {
        applied.cause == DamageCause::Hitscan
    });
}

/// A new life starts with a full magazine.
pub(crate) fn refill_respawned(
    mut respawned: MessageReader<PlayerRespawned>,
    mut players: Query<(&PlayerId, &mut Weapons, &mut RifleState), Without<RetiredEntity>>,
) {
    for message in respawned.read() {
        for (player, mut weapons, mut rifle) in &mut players {
            if *player == message.player
                && let Some(weapon) = weapons.of_mut(WeaponKind::Rifle)
            {
                weapon.ammo = weapon.magazine;
                weapon.reload_ticks_left = 0;
                rifle.cooldown_ticks = 0;
            }
        }
    }
}
