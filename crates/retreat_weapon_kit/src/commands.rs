//! The shared parts of `<weapon>.reload` and `assert.<weapon>_*`.

use crate::player::{actor, character_of, may_fight, refused};
use crate::rules::{WeaponTicks, try_reload};
use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandOutcome, CommandResult};
use retreat_gameplay_api::{PlayerId, WeaponKind, WeaponStatus, Weapons};

/// `<weapon>.reload` (no arguments).
pub fn reload_weapon(
    world: &mut World,
    args: &CommandArgs,
    kind: WeaponKind,
    ticks: WeaponTicks,
) -> CommandResult {
    args.expect_count(0)?;
    let player = actor(world, args)?;
    let Some(character) = character_of(world, player) else {
        return refused(world, args, "the player has no character");
    };
    if !may_fight(world, character) {
        return refused(world, args, "the player is dead or the match is over");
    }
    let Some(mut weapons) = world.get_mut::<Weapons>(character) else {
        return refused(world, args, "the player has no weapon yet");
    };
    let Some(mut next) = weapons.of(kind).copied() else {
        return refused(world, args, "the player does not have this weapon yet");
    };
    if !try_reload(&mut next, ticks) {
        return refused(world, args, "the magazine is full or already reloading");
    }
    weapons.put(next);
    Ok(CommandOutcome::Continue)
}

/// `assert.<weapon>_<what> <player> <expected>`.
pub fn assert_weapon(
    world: &mut World,
    args: &CommandArgs,
    kind: WeaponKind,
    what: &str,
    read: fn(&WeaponStatus) -> u32,
) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: u32 = args.parse(1, what)?;
    let actual = character_of(world, player)
        .and_then(|entity| world.get::<Weapons>(entity))
        .and_then(|weapons| weapons.of(kind))
        .map(read)
        .ok_or_else(|| {
            args.failure(format!(
                "player {} has no {} weapon",
                player.0,
                kind.label()
            ))
        })?;
    if actual != expected {
        return Err(args.failure(format!(
            "player {} has {actual} {what}, expected {expected}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}
