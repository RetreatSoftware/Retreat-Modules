//! When a shot or a reload is allowed. Pure: the same on the server, a client and in a replay.
//! `cooldown` is the weapon module's own state: ticks until the next shot.

use retreat_gameplay_api::{WeaponStatus, ticks_from_seconds};
use std::time::Duration;

/// The server may run two inputs of a jittery client closer together than the client sent them. A
/// shot this many ticks early is accepted, and the missing wait is added to the next one, so the
/// average fire rate cannot exceed the configured one.
pub const LATE_INPUT_TOLERANCE_TICKS: u32 = 3;

/// A weapon's configured times in fixed ticks (each at least 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponTicks {
    pub fire_interval: u32,
    pub reload: u32,
}

impl WeaponTicks {
    pub fn of(fire_interval_seconds: f32, reload_seconds: f32, timestep: Duration) -> Self {
        Self {
            fire_interval: ticks_from_seconds(fire_interval_seconds, timestep).max(1),
            reload: ticks_from_seconds(reload_seconds, timestep).max(1),
        }
    }
}

/// Whether this machine should issue a shot now. Asked before the tick's timers run, for a command
/// that runs after `ticks_until_run` more of them: 1 for a line queued for the network, 0 for one
/// run at once.
pub fn ready_to_send(weapon: &WeaponStatus, cooldown: u32, ticks_until_run: u32) -> bool {
    let loaded = match weapon.reload_ticks_left {
        0 => weapon.ammo > 0,
        left => left <= ticks_until_run,
    };
    cooldown <= ticks_until_run && loaded
}

/// Uses a round if the weapon is ready; false changes nothing.
pub fn try_fire(weapon: &mut WeaponStatus, cooldown: &mut u32, ticks: WeaponTicks) -> bool {
    if *cooldown > LATE_INPUT_TOLERANCE_TICKS
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
    *cooldown += ticks.fire_interval;
    true
}

/// Starts a reload unless one runs or the magazine is full.
pub fn try_reload(weapon: &mut WeaponStatus, ticks: WeaponTicks) -> bool {
    if weapon.reload_ticks_left > 0 || weapon.ammo >= weapon.magazine {
        return false;
    }
    weapon.reload_ticks_left = ticks.reload;
    true
}

/// One tick of cooldown and reload; an empty magazine starts to reload by itself while `alive`.
pub fn advance_weapon(
    weapon: &mut WeaponStatus,
    cooldown: &mut u32,
    ticks: WeaponTicks,
    alive: bool,
) {
    *cooldown = cooldown.saturating_sub(1);
    if weapon.reload_ticks_left > 0 {
        weapon.reload_ticks_left -= 1;
        if weapon.reload_ticks_left == 0 {
            weapon.ammo = weapon.magazine;
        }
    } else if weapon.ammo == 0 && alive {
        try_reload(weapon, ticks);
    }
}
