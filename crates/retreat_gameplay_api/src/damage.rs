//! Damage, death and respawn messages. The server decides all of them.

use crate::{PlayerId, SpawnPoint};
use bevy::prelude::*;

/// What kind of thing hurt a player (kill feed, sounds, rules such as self-damage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageCause {
    /// An instant shot.
    Hitscan,
    /// A projectile hitting the player directly.
    Projectile,
    /// The area of an explosion.
    Blast,
    /// The world itself (falling out of the map).
    World,
}

/// "Hurt this player." Written by whatever deals damage; only the health module acts on it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRequested {
    pub target: PlayerId,
    /// Hit points to remove, already including range falloff or self-damage reduction.
    pub amount: u32,
    /// `None` for the world. Equal to `target` for self-damage.
    pub attacker: Option<PlayerId>,
    pub cause: DamageCause,
}

/// Damage that really happened (not sent for hits on the dead or protected).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageApplied {
    pub target: PlayerId,
    pub amount: u32,
    pub attacker: Option<PlayerId>,
    pub cause: DamageCause,
    /// Hit points left after this damage.
    pub remaining: u32,
}

/// A player lost their last hit point. Sent once per death.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDied {
    pub victim: PlayerId,
    /// Who dealt the last damage; `None` for the world, equal to `victim` for a self-kill.
    pub killer: Option<PlayerId>,
    pub cause: DamageCause,
}

/// A dead player is alive again at `spawn` with full health. Modules reset per-life state
/// (velocity, ammo, camera direction) on it.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct PlayerRespawned {
    pub player: PlayerId,
    pub spawn: SpawnPoint,
}
