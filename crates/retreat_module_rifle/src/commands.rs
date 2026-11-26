//! `rifle.*` and `assert.rifle_*`. Clients may send `rifle.fire` and `rifle.reload`.

use crate::config::RifleConfig;
use crate::state::{RifleShot, RifleState, ticks_of};
use crate::tracer::ShotOutput;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandOutcome, CommandResult, EffectTime, NetLocalClient, RayCast, RetiredEntity,
    cast_ray_at,
};
use retreat_gameplay_api::{
    CharacterSize, DamageCause, DamageRequested, LookAngles, PlayerFired, PlayerId, WeaponKind,
    Weapons, view_direction,
};
use retreat_weapon_kit::{
    actor, assert_weapon, character_of, dead_players, may_fight, refused, reload_weapon, try_fire,
};

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
    let ticks = ticks_of(
        world.resource::<RifleConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    let mut query = world.query::<(&Transform, &mut Weapons, &mut RifleState, &RifleShot)>();
    let Ok((transform, mut weapons, mut rifle, shot)) = query.get_mut(world, shooter) else {
        return refused(world, args, "the player has no rifle yet");
    };
    let (center, count) = (transform.translation, shot.count + 1);
    let Some(mut weapon) = weapons.of(WeaponKind::Rifle).copied() else {
        return refused(world, args, "the player has no rifle yet");
    };
    let mut next = *rifle;
    if !try_fire(&mut weapon, &mut next.cooldown_ticks, ticks) {
        return refused(world, args, "the rifle is not ready");
    }
    weapons.put(weapon);
    rifle.set_if_neq(next);
    world.write_message(PlayerFired { player });
    let from = world.resource::<CharacterSize>().eye(center);
    let aim = LookAngles { yaw, pitch };
    let (to, target) = cast_shot(world, shooter, from, aim, view_tick);
    let shot = RifleShot { count, from, to };
    world.entity_mut(shooter).insert(shot);
    let mut output = SystemState::<ShotOutput>::new(world);
    output
        .get_mut(world)
        .expect("the shot output's resources")
        .play(player, shot, EffectTime::Present);
    // A client only predicts its ammo and the look: hits are the server's to decide.
    if world.resource::<NetLocalClient>().0.is_none()
        && let Some(target) = target
    {
        let amount = world.resource::<RifleConfig>().damage;
        world.write_message(DamageRequested {
            target,
            amount,
            attacker: Some(player),
            cause: DamageCause::Hitscan,
        });
    }
    Ok(CommandOutcome::Continue)
}

/// Casts from the eyes against the others as they were at `view_tick`: where the shot ends, and
/// the player it hit there.
fn cast_shot(
    world: &mut World,
    shooter: Entity,
    from: Vec3,
    aim: LookAngles,
    view_tick: f64,
) -> (Vec3, Option<PlayerId>) {
    let mut exclude = dead_players(world);
    exclude.push(shooter);
    let direction = view_direction(aim);
    let range = world.resource::<RifleConfig>().range;
    let cast = RayCast {
        origin: from,
        direction,
        max_distance: range,
        exclude,
    };
    let Some(hit) = cast_ray_at(world, &cast, view_tick) else {
        return (from + direction * range, None);
    };
    let mut players = world.query_filtered::<&PlayerId, Without<RetiredEntity>>();
    (hit.point, players.get(world, hit.entity).ok().copied())
}

pub(crate) fn reload_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let ticks = ticks_of(
        world.resource::<RifleConfig>(),
        world.resource::<Time<Fixed>>(),
    );
    reload_weapon(world, args, WeaponKind::Rifle, ticks)
}

pub(crate) fn assert_ammo_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, WeaponKind::Rifle, "rounds", |weapon| {
        weapon.ammo
    })
}

pub(crate) fn assert_hits_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    assert_weapon(world, args, WeaponKind::Rifle, "hits", |weapon| {
        weapon.hits_landed
    })
}
