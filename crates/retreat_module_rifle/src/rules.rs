//! When a shot or a reload is allowed. Pure: the same on the server, a client and in a replay.

use crate::config::RifleConfig;
use crate::state::RifleState;
use retreat_gameplay_api::{WeaponStatus, ticks_from_seconds};
use std::time::Duration;

/// The server may run two inputs of a jittery client closer together than the client sent them. A
/// shot this many ticks early is accepted, and the missing wait is added to the next one, so the
/// average fire rate cannot exceed the configured one.
pub const LATE_INPUT_TOLERANCE_TICKS: u32 = 3;

/// The configured times in fixed ticks (each at least 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RifleTicks {
    pub fire_interval: u32,
    pub reload: u32,
}

impl RifleTicks {
    pub fn of(config: &RifleConfig, timestep: Duration) -> Self {
        Self {
            fire_interval: ticks_from_seconds(config.fire_interval_seconds, timestep).max(1),
            reload: ticks_from_seconds(config.reload_seconds, timestep).max(1),
        }
    }
}

/// Whether this machine should issue a shot now. Asked before the tick's timers run, for a command
/// that runs after `ticks_until_run` more of them: 1 for a line queued for the network, 0 for one
/// run at once.
pub fn ready_to_send(weapon: &WeaponStatus, rifle: &RifleState, ticks_until_run: u32) -> bool {
    let loaded = match weapon.reload_ticks_left {
        0 => weapon.ammo > 0,
        left => left <= ticks_until_run,
    };
    rifle.cooldown_ticks <= ticks_until_run && loaded
}

/// Uses a round if the weapon is ready; false changes nothing.
pub fn try_fire(weapon: &mut WeaponStatus, rifle: &mut RifleState, ticks: RifleTicks) -> bool {
    if rifle.cooldown_ticks > LATE_INPUT_TOLERANCE_TICKS
        || weapon.reload_ticks_left > LATE_INPUT_TOLERANCE_TICKS
    {
        return false;
    }
    if weapon.reload_ticks_left > 0 {
        weapon.reload_ticks_left = 0;
        weapon.ammo = weapon.magazine;
    }
    if weapon.ammo == 0 {
        return false;
    }
    weapon.ammo -= 1;
    rifle.cooldown_ticks += ticks.fire_interval;
    true
}

/// Starts a reload unless one runs or the magazine is full.
pub fn try_reload(weapon: &mut WeaponStatus, ticks: RifleTicks) -> bool {
    if weapon.reload_ticks_left > 0 || weapon.ammo >= weapon.magazine {
        return false;
    }
    weapon.reload_ticks_left = ticks.reload;
    true
}
