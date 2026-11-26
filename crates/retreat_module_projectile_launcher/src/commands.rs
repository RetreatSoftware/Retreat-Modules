//! `launcher.*` and `assert.launcher_*`. Clients may send `launcher.fire` and `launcher.reload`.

use crate::config::LauncherConfig;
use crate::state::{LauncherState, Rocket, ticks_of};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandOutcome, CommandResult, NetPose, RetiredEntity, spawn_predicted,
};
use retreat_gameplay_api::{
    CharacterSize, LookAngles, PlayerFired, WeaponKind, Weapons, view_direction,
};
use retreat_weapon_kit::{
    actor, assert_weapon, character_of, may_fight, refused, reload_weapon, try_fire,
};

pub(crate) fn fire_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let yaw: f32 = args.parse(0, "yaw")?;
    let pitch: f32 = args.parse(1, "pitch")?;
    if !(yaw.is_finite() && pitch.is_finite()) {
        return Err(args.failure("the numbers must be finite"));
    }
    let player = actor(world, args)?;
    let Some(shooter) = character_of(world, player) else {
        return refused(world, args, "the player has no character");
    };
    if !may_fight(world, shooter) {
        return refused(world, args, "the player is dead or the match is over");
    }
    let ticks = ticks_of(
        world.resource::<LauncherConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    let mut query = world.query::<(&Transform, &mut Weapons, &mut LauncherState)>();
    let Ok((transform, mut weapons, mut launcher)) = query.get_mut(world, shooter) else {
        return refused(world, args, "the player has no launcher yet");
    };
    let center = transform.translation;
    let Some(mut weapon) = weapons.of(WeaponKind::Launcher).copied() else {
        return refused(world, args, "the player has no launcher yet");
    };
    let mut next = *launcher;
    if !try_fire(&mut weapon, &mut next.cooldown_ticks, ticks) {
        return refused(world, args, "the launcher is not ready");
    }
    weapons.put(weapon);
    launcher.set_if_neq(next);
    world.write_message(PlayerFired { player });
    let eye = world.resource::<CharacterSize>().eye(center);
    spawn_predicted(
        world,
        "rocket",
        (
            Rocket {
                owner: player,
                direction: view_direction(LookAngles { yaw, pitch }),
                age: 0,
                exploded: 0,
            },
            NetPose::at(eye),
        ),
    );
    Ok(CommandOutcome::Continue)
}

pub(crate) fn reload_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let ticks = ticks_of(
        world.resource::<LauncherConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    reload_weapon(world, args, WeaponKind::Launcher, ticks)
}

pub(crate) fn assert_ammo_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, WeaponKind::Launcher, "rockets", |weapon| {
        weapon.ammo
    })
}

pub(crate) fn assert_hits_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, WeaponKind::Launcher, "hits", |weapon| {
        weapon.hits_landed
    })
}

/// Rockets in flight (exploded ones that linger do not count).
pub(crate) fn assert_rockets_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: usize = args.parse(0, "flying")?;
    let flying = world
        .query_filtered::<&Rocket, Without<RetiredEntity>>()
        .iter(world)
        .filter(|rocket| rocket.exploded == 0)
        .count();
    if flying != expected {
        return Err(args.failure(format!("{flying} rockets are flying, expected {expected}")));
    }
    Ok(CommandOutcome::Continue)
}
