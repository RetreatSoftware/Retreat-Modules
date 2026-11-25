//! `rifle.*` and `assert.rifle_*`. Clients may send `rifle.fire` and `rifle.reload`.

use crate::config::RifleConfig;
use crate::rules::{RifleTicks, try_fire, try_reload};
use crate::state::RifleState;
use crate::targets::{character_of, dead_players, may_fight};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, NetCommandSender, NetLocalClient,
    RayCast, RetiredEntity, cast_ray_at,
};
use retreat_gameplay_api::{
    CHARACTER_EYE_HEIGHT, DamageCause, DamageRequested, LookAngles, PlayerFired, PlayerId,
    SoloPlayer, WeaponStatus, character_feet, local_player_id, view_direction,
};

/// The player a command acts for: who sent it, otherwise this machine's player.
fn actor(world: &World, args: &CommandArgs) -> Result<PlayerId, CommandError> {
    if let Some(id) = world.resource::<NetCommandSender>().0 {
        return Ok(PlayerId(id));
    }
    local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )
    .ok_or_else(|| {
        args.failure("this machine has no player; send it from a client or use spawn.solo")
    })
}

/// A refusal: normal for a network input (it can cross a death or a reload), an error in a script.
fn refused(world: &World, args: &CommandArgs, why: &str) -> CommandResult {
    if world.resource::<NetCommandSender>().0.is_some() {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(why.to_string()))
    }
}

pub(crate) fn fire_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let has_view = !args.rest(2).is_empty();
    args.expect_count(if has_view { 3 } else { 2 })?;
    let yaw: f32 = args.parse(0, "yaw")?;
    let pitch: f32 = args.parse(1, "pitch")?;
    let view_tick: f64 = if has_view {
        args.parse(2, "view_tick")?
    } else {
        0.0
    };
    if !(yaw.is_finite() && pitch.is_finite() && view_tick.is_finite()) {
        return Err(args.failure("the numbers must be finite"));
    }
    let player = actor(world, args)?;
    let Some(shooter) = character_of(world, player) else {
        return refused(world, args, "the player has no character");
    };
    if !may_fight(world, shooter) {
        return refused(world, args, "the player is dead or the match is over");
    }
    let ticks = RifleTicks::of(
        world.resource::<RifleConfig>(),
        world.resource::<Time<Fixed>>().timestep(),
    );
    let mut weapons = world.query::<(&mut WeaponStatus, &mut RifleState)>();
    let Ok((mut weapon, mut rifle)) = weapons.get_mut(world, shooter) else {
        return refused(world, args, "the player has no rifle yet");
    };
    let (mut next_weapon, mut next_rifle) = (*weapon, *rifle);
    if !try_fire(&mut next_weapon, &mut next_rifle, ticks) {
        return refused(world, args, "the rifle is not ready");
    }
    weapon.set_if_neq(next_weapon);
    rifle.set_if_neq(next_rifle);
    world.write_message(PlayerFired { player });
    // A client only predicts its ammo: hits are the server's to decide.
    if world.resource::<NetLocalClient>().0.is_none() {
        judge_shot(world, player, shooter, LookAngles { yaw, pitch }, view_tick);
    }
    Ok(CommandOutcome::Continue)
}

/// Casts from the eyes against the others as they were at `view_tick`.
fn judge_shot(
    world: &mut World,
    player: PlayerId,
    shooter: Entity,
    aim: LookAngles,
    view_tick: f64,
) {
    let Some(center) = world.get::<Transform>(shooter).map(|t| t.translation) else {
        return;
    };
    let mut exclude = dead_players(world);
    exclude.push(shooter);
    let config = world.resource::<RifleConfig>();
    let (damage, range) = (config.damage, config.range);
    let cast = RayCast {
        origin: character_feet(center) + Vec3::Y * CHARACTER_EYE_HEIGHT,
        direction: view_direction(aim),
        max_distance: range,
        exclude,
    };
    let Some(hit) = cast_ray_at(world, &cast, view_tick) else {
        return;
    };
    let mut targets = world.query_filtered::<&PlayerId, Without<RetiredEntity>>();
    let Ok(target) = targets.get(world, hit.entity).copied() else {
        return;
    };
    world.write_message(DamageRequested {
        target,
        amount: damage,
        attacker: Some(player),
        cause: DamageCause::Hitscan,
    });
}

pub(crate) fn reload_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let player = actor(world, args)?;
    let Some(character) = character_of(world, player) else {
        return refused(world, args, "the player has no character");
    };
    if !may_fight(world, character) {
        return refused(world, args, "the player is dead or the match is over");
    }
    let ticks = RifleTicks::of(
        world.resource::<RifleConfig>(),
        world.resource::<Time<Fixed>>().timestep(),
    );
    let Some(mut weapon) = world.get_mut::<WeaponStatus>(character) else {
        return refused(world, args, "the player has no rifle yet");
    };
    let mut next = *weapon;
    if !try_reload(&mut next, ticks) {
        return refused(world, args, "the magazine is full or already reloading");
    }
    *weapon = next;
    Ok(CommandOutcome::Continue)
}

pub(crate) fn assert_ammo_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, "rounds", |weapon| weapon.ammo)
}

pub(crate) fn assert_hits_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, "hits", |weapon| weapon.hits_landed)
}

fn assert_weapon(
    world: &mut World,
    args: &CommandArgs,
    what: &str,
    read: fn(&WeaponStatus) -> u32,
) -> CommandResult {
    args.expect_count(2)?;
    let player = PlayerId(args.parse(0, "player")?);
    let expected: u32 = args.parse(1, what)?;
    let actual = character_of(world, player)
        .and_then(|entity| world.get::<WeaponStatus>(entity))
        .map(read)
        .ok_or_else(|| args.failure(format!("player {} has no rifle", player.0)))?;
    if actual != expected {
        return Err(args.failure(format!(
            "player {} has {actual} {what}, expected {expected}",
            player.0
        )));
    }
    Ok(CommandOutcome::Continue)
}
