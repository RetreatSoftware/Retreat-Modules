//! A weapon's entry in the contract's `Weapons` (one component per player, shared by all weapons).

use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{DamageApplied, PlayerId, WeaponKind, WeaponStatus, Weapons};

/// A full weapon that never hit anything.
pub fn loaded(kind: WeaponKind, magazine: u32) -> WeaponStatus {
    WeaponStatus {
        kind,
        ammo: magazine,
        magazine,
        reload_ticks_left: 0,
        hits_landed: 0,
    }
}

/// Gives the player the weapon. Queued on the entity, so several weapon modules can equip the same
/// player in one tick.
pub fn equip(commands: &mut Commands, player: Entity, status: WeaponStatus) {
    commands
        .entity(player)
        .queue(move |mut player: EntityWorldMut| {
            let mut weapons = player.get::<Weapons>().cloned().unwrap_or_default();
            weapons.put(status);
            player.insert(weapons);
        });
}

/// Raises `hits_landed` of the attacker's weapon for every damage that `counts` picks and that
/// really hurt another player.
pub fn count_hits(
    damage: &mut MessageReader<DamageApplied>,
    players: &mut Query<(&PlayerId, &mut Weapons), Without<RetiredEntity>>,
    kind: WeaponKind,
    counts: fn(&DamageApplied) -> bool,
) {
    for applied in damage.read() {
        if !counts(applied) || applied.amount == 0 {
            continue;
        }
        for (player, mut weapons) in players.iter_mut() {
            if applied.attacker == Some(*player)
                && applied.target != *player
                && let Some(weapon) = weapons.of_mut(kind)
            {
                weapon.hits_landed += 1;
            }
        }
    }
}
