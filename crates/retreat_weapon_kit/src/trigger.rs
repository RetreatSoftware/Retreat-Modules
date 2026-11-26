//! This machine's fire and reload buttons and aim. The camera looks over a shoulder and the shot
//! leaves the eyes, so the eyes are aimed at what the center of the screen shows.

use crate::player::{character_of, dead_players, may_fight};
use crate::rules::ready_to_send;
use bevy::math::ops;
use bevy::prelude::*;
use retreat_engine_api::{
    GameCamera, InputActions, NetInputQueue, NetLocalClient, NetViewTick, RayCast,
    RemoteInterpolation, cast_ray_at, run_command_line,
};
use retreat_gameplay_api::{
    CharacterSize, LookAngles, SoloPlayer, WeaponKind, Weapons, local_player_id,
};

/// What a weapon module asks with, once per tick.
#[derive(Debug, Clone, Copy)]
pub struct Trigger {
    pub kind: WeaponKind,
    /// Action id of the fire button (held: automatic fire).
    pub fire_action: &'static str,
    /// Action id of the reload button (acts on the press).
    pub reload_action: &'static str,
    /// How far the weapon aims, metres.
    pub range: f32,
}

/// What to send now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerOrder {
    Reload,
    Fire {
        aim: LookAngles,
        /// The server tick the others are drawn at, or 0 for "now".
        view_tick: f64,
    },
}

/// `cooldown` is the local character's weapon cooldown (`None` before it has the weapon).
/// Call it from `FixedUpdate`, never during a replay: this tick's buttons are not a replay's.
pub fn pull_trigger(
    world: &mut World,
    trigger: Trigger,
    cooldown: fn(&World, Entity) -> Option<u32>,
    reload_was_down: &mut bool,
) -> Option<TriggerOrder> {
    let actions = world.resource::<InputActions>();
    let fire = actions.is_held(trigger.fire_action);
    let reload = actions.is_held(trigger.reload_action);
    let reload_pressed = reload && !*reload_was_down;
    *reload_was_down = reload;
    if !fire && !reload_pressed {
        return None;
    }
    let local = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )?;
    let character = character_of(world, local)?;
    if !may_fight(world, character) {
        return None;
    }
    let weapon = *world.get::<Weapons>(character)?.of(trigger.kind)?;
    let cooldown = cooldown(world, character)?;
    // A queued line runs at the start of the next tick, after this tick's timers.
    let ticks_until_run = u32::from(world.resource::<NetInputQueue>().accepting);
    if reload_pressed && weapon.reload_ticks_left == 0 && weapon.ammo < weapon.magazine {
        Some(TriggerOrder::Reload)
    } else if fire && ready_to_send(&weapon, cooldown, ticks_until_run) {
        let view_tick = view_tick(world, character);
        let aim = aim_of(world, character, trigger.range, view_tick)?;
        Some(TriggerOrder::Fire { aim, view_tick })
    } else {
        None
    }
}

/// Sends the line to the server, or runs it here without a network.
pub fn issue(world: &mut World, line: String) {
    if world.resource::<NetInputQueue>().accepting {
        world.resource_mut::<NetInputQueue>().push(line);
    } else if let Err(error) = run_command_line(world, &line) {
        warn!("{error}");
    }
}

/// The server tick the others are drawn at, or 0 ("now") when this machine draws nobody from the
/// server's past (no looks module, or no network): a rewind would then miss what is on screen.
fn view_tick(world: &mut World, own: Entity) -> f64 {
    let mut others = world.query_filtered::<Entity, With<RemoteInterpolation>>();
    if others.iter(world).any(|other| other != own) {
        world.resource::<NetViewTick>().for_input()
    } else {
        0.0
    }
}

/// From the eyes to what the center of the camera shows; without a camera, the player's view.
fn aim_of(world: &mut World, own: Entity, range: f32, view_tick: f64) -> Option<LookAngles> {
    let center = world.get::<Transform>(own)?.translation;
    let eye = world.resource::<CharacterSize>().eye(center);
    let mut cameras = world.query::<(&Name, &GameCamera)>();
    let camera = cameras
        .iter(world)
        .min_by(|a, b| a.0.as_str().cmp(b.0.as_str()))
        .map(|(_, camera)| camera.clone());
    let Some(camera) = camera else {
        return world.get::<LookAngles>(own).copied();
    };
    let forward = (camera.target - camera.position).try_normalize()?;
    // Starts level with the eyes: what is between the camera and the player is not a target.
    let start = camera.position + forward * (eye - camera.position).dot(forward).max(0.0);
    let mut exclude = dead_players(world);
    exclude.push(own);
    let cast = RayCast {
        origin: start,
        direction: forward,
        max_distance: range,
        exclude,
    };
    let target =
        cast_ray_at(world, &cast, view_tick).map_or(start + forward * range, |hit| hit.point);
    aim_angles(eye, target)
}

/// The view angles from `eye` towards `target`; `None` when they are the same point.
pub fn aim_angles(eye: Vec3, target: Vec3) -> Option<LookAngles> {
    let direction = (target - eye).try_normalize()?;
    Some(LookAngles {
        yaw: ops::atan2(-direction.x, -direction.z),
        pitch: ops::asin(direction.y.clamp(-1.0, 1.0)),
    })
}
