//! The launcher's rolled-back and replicated state, and the systems that advance it.

use crate::config::LauncherConfig;
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, NetState, RemoveNetEntity, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{
    AmmoSource, DamageApplied, DamageCause, Health, Inventory, ItemCatalog, MatchRestarted,
    PlayerId, PlayerRespawned, SoloPlayer, WeaponKind, Weapons, local_player_id,
};
use retreat_weapon_kit::{WeaponTicks, advance_weapon, equip, loaded, play_reload};
use serde::{Deserialize, Serialize};

/// On a player: what only the launcher needs; ammo and reload are in the contract's `Weapons`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherState {
    /// Ticks until the next rocket.
    pub cooldown_ticks: u32,
    /// Rockets fired since the player appeared.
    pub shots: u32,
}

impl NetState for LauncherState {
    const ID: &'static str = "projectile_launcher.state";
}

/// A rocket, on its own entity; where it is is its `NetPose`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rocket {
    pub owner: PlayerId,
    /// Unit vector of the flight.
    pub direction: Vec3,
    /// Ticks it has flown.
    pub age: u32,
    /// Ticks since it exploded; 0 while it flies.
    pub exploded: u32,
}

impl NetState for Rocket {
    const ID: &'static str = "projectile_launcher.rocket";
}

pub(crate) fn ticks_of(config: &LauncherConfig, time: &Time<Fixed>) -> WeaponTicks {
    WeaponTicks::of(
        config.fire_interval_seconds,
        config.reload_seconds,
        time.timestep(),
    )
}

type Unarmed = (
    With<PlayerId>,
    Without<LauncherState>,
    Without<RetiredEntity>,
);

/// Every player gets a loaded launcher.
pub(crate) fn attach_launchers(
    players: Query<Entity, Unarmed>,
    config: Res<LauncherConfig>,
    mut commands: Commands,
) {
    for entity in &players {
        commands.entity(entity).insert(LauncherState::default());
        equip(
            &mut commands,
            entity,
            loaded(WeaponKind::Launcher, config.magazine),
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
    mut players: Query<WeaponParts<LauncherState>, Without<RetiredEntity>>,
    catalog: Res<ItemCatalog>,
    config: Res<LauncherConfig>,
    time: Res<Time<Fixed>>,
) {
    let ticks = ticks_of(&config, &time);
    for (mut weapons, mut launcher, health, inventory) in &mut players {
        let Some(mut weapon) = weapons.of(WeaponKind::Launcher).copied() else {
            continue;
        };
        let mut next = *launcher;
        let alive = health.is_none_or(Health::is_alive);
        let mut carried = inventory.as_deref().cloned();
        let mut source = AmmoSource {
            inventory: carried.as_mut(),
            catalog: &catalog,
            weapon: WeaponKind::Launcher,
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
        launcher.set_if_neq(next);
        if weapons.of(WeaponKind::Launcher) != Some(&weapon) {
            weapons.put(weapon);
        }
    }
}

/// Counts the rockets that really hurt another player.
pub(crate) fn count_hits(
    mut damage: MessageReader<DamageApplied>,
    mut players: Query<(&PlayerId, &mut Weapons), Without<RetiredEntity>>,
) {
    retreat_weapon_kit::count_hits(&mut damage, &mut players, WeaponKind::Launcher, |applied| {
        matches!(applied.cause, DamageCause::Projectile | DamageCause::Blast)
    });
}

/// A new life starts with a full magazine.
pub(crate) fn refill_respawned(
    mut respawned: MessageReader<PlayerRespawned>,
    mut players: Query<(&PlayerId, &mut Weapons, &mut LauncherState), Without<RetiredEntity>>,
) {
    for message in respawned.read() {
        for (player, mut weapons, mut launcher) in &mut players {
            if *player == message.player
                && let Some(weapon) = weapons.of_mut(WeaponKind::Launcher)
            {
                weapon.ammo = weapon.magazine;
                weapon.reload_ticks_left = 0;
                launcher.cooldown_ticks = 0;
            }
        }
    }
}

/// A new match starts without the old one's rockets.
pub(crate) fn remove_rockets(
    mut restarts: MessageReader<MatchRestarted>,
    rockets: Query<Entity, (With<Rocket>, Without<RetiredEntity>)>,
    mut commands: Commands,
) {
    if restarts.read().count() == 0 {
        return;
    }
    for rocket in &rockets {
        commands.entity(rocket).insert(RemoveNetEntity);
    }
}

/// This machine's player hears its launcher reload.
pub(crate) fn play_own_reload(
    players: Query<(&PlayerId, &Weapons, &LauncherState), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    config: Res<LauncherConfig>,
    mut sounds: ResMut<SoundEvents>,
) {
    let local = local_player_id(*client, *solo);
    for (player, weapons, launcher) in &players {
        if Some(*player) == local
            && let Some(weapon) = weapons.of(WeaponKind::Launcher)
        {
            let cue = config.reload_sound.as_ref();
            play_reload(&mut sounds, cue, *player, weapon, launcher.shots);
        }
    }
}
