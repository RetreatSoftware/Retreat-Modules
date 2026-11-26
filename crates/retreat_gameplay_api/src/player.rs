//! Who a player is and how big a player character is.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// Most players in one match.
pub const MAX_PLAYERS: usize = 8;

/// How big every player character of the game is, in metres: its body (an upright capsule), what a
/// shot can hit and where it looks from. A game with smaller characters inserts its own before
/// adding any module (the same on the server and every client). The default is a human.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CharacterSize {
    /// Radius of the capsule.
    pub radius: f32,
    /// Total height from feet to top; at least twice the radius.
    pub height: f32,
    /// Height of the eyes above the feet.
    pub eye_height: f32,
}

impl Default for CharacterSize {
    fn default() -> Self {
        Self {
            radius: 0.35,
            height: 1.8,
            eye_height: 1.6,
        }
    }
}

impl CharacterSize {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.radius.is_finite() && self.radius > 0.0) {
            return Err(format!("radius must be more than 0, got {}", self.radius));
        }
        if !(self.height.is_finite() && self.height >= 2.0 * self.radius) {
            return Err(format!(
                "height must be at least twice the radius, got {}",
                self.height
            ));
        }
        if !(0.0..=self.height).contains(&self.eye_height) {
            return Err(format!(
                "eye_height must be within the character (0 to {}), got {}",
                self.height, self.eye_height
            ));
        }
        Ok(())
    }

    /// Half the height of the straight part of the capsule.
    pub fn capsule_half_height(&self) -> f32 {
        self.height / 2.0 - self.radius
    }

    /// A character entity's `Transform` is the middle of its body. This is it for feet at `feet`.
    pub fn center(&self, feet: Vec3) -> Vec3 {
        feet + Vec3::Y * (self.height / 2.0)
    }

    /// Where the feet of a character with its `Transform` at `center` are.
    pub fn feet(&self, center: Vec3) -> Vec3 {
        center - Vec3::Y * (self.height / 2.0)
    }

    /// Where the eyes of a character with its `Transform` at `center` are.
    pub fn eye(&self, center: Vec3) -> Vec3 {
        self.feet(center) + Vec3::Y * self.eye_height
    }
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
