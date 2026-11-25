//! What a player's weapon shows: written by the weapon module, read by the HUD.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// On a player that carries a weapon.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponStatus {
    /// Shots left in the magazine.
    pub ammo: u32,
    pub magazine: u32,
    /// Ticks until a running reload is done; 0 when not reloading.
    pub reload_ticks_left: u32,
    /// Shots of this player that really hurt someone, as the server counts them. It only grows, so
    /// a reader shows a hit marker when it rises.
    pub hits_landed: u32,
}

impl NetState for WeaponStatus {
    const ID: &'static str = "gameplay.weapon";
}
