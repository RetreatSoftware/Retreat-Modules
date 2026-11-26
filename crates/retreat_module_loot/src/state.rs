//! A pickup's rolled-back and replicated state; where it is is its `NetPose`.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use retreat_gameplay_api::ItemStack;
use serde::{Deserialize, Serialize};

/// Where a pickup comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickupOrigin {
    /// A spot of the map; there while no time is left until it is.
    Spot {
        kind: String,
        respawn_ticks_left: u32,
    },
    /// What a player lost; gone when no time is left.
    Drop { ticks_left: u32 },
}

/// On every pickup entity.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pickup {
    pub contents: Vec<ItemStack>,
    pub origin: PickupOrigin,
}

impl NetState for Pickup {
    const ID: &'static str = "loot.pickup";
}

impl Pickup {
    /// Whether it can be seen and taken.
    pub fn is_there(&self) -> bool {
        !matches!(
            self.origin,
            PickupOrigin::Spot {
                respawn_ticks_left: 1..,
                ..
            }
        )
    }
}
