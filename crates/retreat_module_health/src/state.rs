//! The module's own rolled-back and replicated timers, in fixed ticks.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// On a dead player: ticks until it respawns.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RespawnTimer {
    pub ticks_left: u32,
}

impl NetState for RespawnTimer {
    const ID: &'static str = "health.respawn";
}

/// On a freshly spawned player: damage is ignored for this many more ticks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnProtection {
    pub ticks_left: u32,
}

impl NetState for SpawnProtection {
    const ID: &'static str = "health.protection";
}
