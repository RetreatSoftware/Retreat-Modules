//! Putting a respawned character at its spawn point.

use crate::state::MovementState;
use bevy::prelude::*;
use retreat_engine_api::{PhysicsTeleport, PhysicsVelocitySet, RetiredEntity};
use retreat_gameplay_api::{LookAngles, PlayerId, PlayerRespawned, character_center};

pub(crate) fn teleport_respawned(
    mut respawns: MessageReader<PlayerRespawned>,
    mut players: Query<
        (Entity, &PlayerId, &mut MovementState, &mut LookAngles),
        Without<RetiredEntity>,
    >,
    mut commands: Commands,
) {
    for message in respawns.read() {
        for (entity, player, mut state, mut look) in &mut players {
            if *player != message.player {
                continue;
            }
            // A new life: no dash or jump carries over. The held input stays.
            *state = MovementState {
                input: state.input,
                ..default()
            };
            *look = LookAngles {
                yaw: message.spawn.yaw,
                pitch: 0.0,
            };
            // The velocity this tick's movement asked for belongs to the old place.
            commands
                .entity(entity)
                .remove::<PhysicsVelocitySet>()
                .insert(PhysicsTeleport(character_center(message.spawn.feet)));
        }
    }
}
