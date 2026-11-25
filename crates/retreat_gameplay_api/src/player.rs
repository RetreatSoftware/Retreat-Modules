//! Who a player is and how big a player character is.

use bevy::prelude::*;

/// Most players in one match.
pub const MAX_PLAYERS: usize = 8;

/// Radius of an upright character (a capsule), in metres.
pub const CHARACTER_RADIUS: f32 = 0.35;
/// Total height of a character from feet to top, in metres.
pub const CHARACTER_HEIGHT: f32 = 1.8;
/// Height of a character's eyes above its feet, in metres.
pub const CHARACTER_EYE_HEIGHT: f32 = 1.6;

/// Identifies a player for the whole session; the value is the network client id. The player's
/// character entity carries it as a component and is named [`player_entity_name`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u32);

/// The entity name of a player's character: `player_<id>`.
pub fn player_entity_name(player: PlayerId) -> String {
    format!("player_{}", player.0)
}
