//! This machine's keys and mouse, turned into its player's input.

use crate::config::MovementConfig;
use crate::state::{MovementState, PlayerInput};
use crate::{ADVANCE_AXIS, STRAFE_AXIS};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use retreat_engine_api::{
    AnalogInput, InputActions, NetInputQueue, NetLocalClient, NetReplaying, RetiredEntity,
};
use retreat_gameplay_api::{LookAngles, PlayerId, PlayerRespawned, SoloPlayer, local_player_id};

/// The view this machine's mouse has turned to. `None` until its character exists. Not shared
/// state: it travels to the simulation inside every input.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct LocalAim(pub Option<LookAngles>);

/// What this machine's devices say, and who they control.
#[derive(SystemParam)]
pub(crate) struct Devices<'w> {
    analog: Res<'w, AnalogInput>,
    actions: Res<'w, InputActions>,
    client: Res<'w, NetLocalClient>,
    solo: Res<'w, SoloPlayer>,
    replaying: Res<'w, NetReplaying>,
}

pub(crate) fn send_input(
    devices: Devices,
    config: Res<MovementConfig>,
    mut respawns: MessageReader<PlayerRespawned>,
    mut aim: ResMut<LocalAim>,
    mut queue: ResMut<NetInputQueue>,
    mut players: Query<(&PlayerId, &mut MovementState, &mut LookAngles), Without<RetiredEntity>>,
) {
    let respawns: Vec<PlayerRespawned> = respawns.read().copied().collect();
    // A replay runs the recorded inputs; this tick's mouse movement is not new.
    if devices.replaying.0 {
        return;
    }
    let Some(local) = local_player_id(*devices.client, *devices.solo) else {
        return;
    };
    let Some((_, mut state, mut look)) = players.iter_mut().find(|(id, ..)| **id == local) else {
        aim.0 = None;
        return;
    };
    let mut view = aim.0.unwrap_or(*look);
    if let Some(respawn) = respawns.iter().rev().find(|r| r.player == local) {
        view = LookAngles {
            yaw: respawn.spawn.yaw,
            pitch: 0.0,
        };
    }
    let mouse = devices.analog.look() * config.mouse_sensitivity;
    // Kept within one turn so the number stays precise over a long session.
    view.yaw = (view.yaw - mouse.x).rem_euclid(std::f32::consts::TAU);
    view.pitch = (view.pitch - mouse.y).clamp(-config.max_pitch, config.max_pitch);
    aim.0 = Some(view);
    let input = PlayerInput {
        strafe: devices.analog.axis(STRAFE_AXIS),
        advance: devices.analog.axis(ADVANCE_AXIS),
        sprint: devices.actions.is_held("movement_sprint"),
        jump: devices.actions.is_held("movement_jump"),
        dash: devices.actions.is_held("movement_dash"),
    };
    if queue.accepting {
        queue.push(input_line(input, view));
    } else {
        state.input = input;
        *look = view;
    }
}

/// The `movement.input` line of an input. An `f32` printed with `{}` parses back to the same bits.
pub(crate) fn input_line(input: PlayerInput, view: LookAngles) -> String {
    format!(
        "movement.input {} {} {} {} {} {} {}",
        input.strafe,
        input.advance,
        view.yaw,
        view.pitch,
        u8::from(input.sprint),
        u8::from(input.jump),
        u8::from(input.dash),
    )
}
