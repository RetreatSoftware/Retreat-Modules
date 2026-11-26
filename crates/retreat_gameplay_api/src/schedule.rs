//! The order of gameplay within one fixed tick.
//!
//! Messages are not rewound by the engine, so every gameplay message must be written and read in the
//! same tick: the writer's system is in an earlier set than the reader's. Readers run every tick and
//! read all messages, even when they then ignore them.

use bevy::prelude::*;

/// Sets of `FixedUpdate`, in this order, all after the tick's commands and input.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySystems {
    /// Players act: weapons and hazards write [`crate::DamageRequested`].
    Act,
    /// Health applies damage, runs respawn timers; writes [`crate::DamageApplied`],
    /// [`crate::PlayerDied`] and [`crate::PlayerRespawned`].
    Damage,
    /// Things in the world are picked up: they write [`crate::GrantRequested`].
    Pickup,
    /// Inventories take in grants and lose their items to deaths; writes [`crate::ItemsLost`].
    Items,
    /// What was lost appears in the world.
    Drops,
    /// The match counts deaths and time; writes [`crate::MatchEnded`] and
    /// [`crate::MatchRestarted`].
    Match,
    /// Modules with per-match state react to [`crate::MatchRestarted`] (health respawns everyone and
    /// writes [`crate::PlayerRespawned`]).
    Restart,
    /// Modules with per-life state react to [`crate::PlayerRespawned`].
    Respawned,
}
