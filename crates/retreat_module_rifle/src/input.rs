//! This machine's fire and reload buttons, turned into command lines.

use crate::aim::aim_angles;
use crate::config::RifleConfig;
use crate::rules::ready_to_send;
use crate::state::RifleState;
use crate::targets::{character_of, dead_players, may_fight};
use bevy::prelude::*;
use retreat_engine_api::{
    GameCamera, InputActions, NetInputQueue, NetLocalClient, NetReplaying, NetViewTick, RayCast,
    RemoteInterpolation, cast_ray_at, run_command_line,
};
use retreat_gameplay_api::{
    CHARACTER_EYE_HEIGHT, LookAngles, SoloPlayer, WeaponStatus, character_feet, local_player_id,
};

pub(crate) fn send_input(world: &mut World, mut reload_was_down: Local<bool>) {
    // A replay runs the recorded inputs; this tick's buttons are not new.
    if world.resource::<NetReplaying>().0 {
        return;
    }
    let actions = world.resource::<InputActions>();
    let fire = actions.is_held("rifle_fire");
    let reload = actions.is_held("rifle_reload");
    let reload_pressed = reload && !*reload_was_down;
    *reload_was_down = reload;
    if !fire && !reload_pressed {
        return;
    }
    let Some(local) = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    ) else {
        return;
    };
    let Some(character) = character_of(world, local) else {
        return;
    };
    if !may_fight(world, character) {
        return;
    }
    let (Some(weapon), Some(rifle)) = (
        world.get::<WeaponStatus>(character).copied(),
        world.get::<RifleState>(character).copied(),
    ) else {
        return;
    };
    // A queued line runs at the start of the next tick, after this tick's timers.
    let ticks_until_run = u32::from(world.resource::<NetInputQueue>().accepting);
    if reload_pressed && weapon.reload_ticks_left == 0 && weapon.ammo < weapon.magazine {
        issue(world, "rifle.reload".to_string());
    } else if fire && ready_to_send(&weapon, &rifle, ticks_until_run) {
        let view_tick = view_tick(world, character);
        if let Some(aim) = aim_of(world, character, view_tick) {
            issue(
                world,
                format!("rifle.fire {} {} {view_tick}", aim.yaw, aim.pitch),
            );
        }
    }
}

/// Sends the line to the server, or runs it here without a network.
fn issue(world: &mut World, line: String) {
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
fn aim_of(world: &mut World, own: Entity, view_tick: f64) -> Option<LookAngles> {
    let center = world.get::<Transform>(own)?.translation;
    let eye = character_feet(center) + Vec3::Y * CHARACTER_EYE_HEIGHT;
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
    let range = world.resource::<RifleConfig>().range;
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
