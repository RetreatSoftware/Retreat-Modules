//! Which player is controlled and watched on this machine.

use crate::PlayerId;
use bevy::prelude::*;
use retreat_engine_api::NetLocalClient;

/// The local player of a game without a network. `None` on a server and on a network client (a
/// client's own player is its client id). Not shared state: it describes this machine.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SoloPlayer(pub Option<PlayerId>);

/// This machine's player: the network client's own, otherwise the solo player. A server has none.
pub fn local_player_id(client: NetLocalClient, solo: SoloPlayer) -> Option<PlayerId> {
    client.0.map(PlayerId).or(solo.0)
}
