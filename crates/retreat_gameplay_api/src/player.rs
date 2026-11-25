//! Who a player is and how big a player character is.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// Most players in one match.
pub const MAX_PLAYERS: usize = 8;

/// Radius of an upright character (a capsule), in metres.
pub const CHARACTER_RADIUS: f32 = 0.35;
/// Total height of a character from feet to top, in metres.
pub const CHARACTER_HEIGHT: f32 = 1.8;
/// Height of a character's eyes above its feet, in metres.
pub const CHARACTER_EYE_HEIGHT: f32 = 1.6;

/// Half the height of the straight part of the character capsule.
pub const CHARACTER_CAPSULE_HALF_HEIGHT: f32 = CHARACTER_HEIGHT / 2.0 - CHARACTER_RADIUS;

/// A character entity's `Transform` is the middle of its body. This is it for feet at `feet`.
pub fn character_center(feet: Vec3) -> Vec3 {
    feet + Vec3::Y * (CHARACTER_HEIGHT / 2.0)
}

/// Where the feet of a character with its `Transform` at `center` are.
pub fn character_feet(center: Vec3) -> Vec3 {
    center - Vec3::Y * (CHARACTER_HEIGHT / 2.0)
}

/// Identifies a player for the whole session; the value is the network client id. The player's
/// character entity carries it as a component and is named [`player_entity_name`].
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PlayerId(pub u32);

impl NetState for PlayerId {
    const ID: &'static str = "gameplay.player";
}

/// The entity name of a player's character: `player_<id>`.
pub fn player_entity_name(player: PlayerId) -> String {
    format!("player_{}", player.0)
}
