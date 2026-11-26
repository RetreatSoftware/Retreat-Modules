//! The module's own rolled-back and replicated timers, in fixed ticks.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// On a freshly spawned player: damage is ignored for this many more ticks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnProtection {
    pub ticks_left: u32,
}

impl NetState for SpawnProtection {
    const ID: &'static str = "health.protection";
}

/// How often the player came back: what tells one death or respawn from the next.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Life(pub u32);

impl NetState for Life {
    const ID: &'static str = "health.life";
}
