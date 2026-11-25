//! What the HUD shows for a given state. Pure.

use retreat_gameplay_api::{Health, RespawnTimer, WeaponStatus};
use std::time::Duration;

/// What this machine's player has; each part is absent when no module provides it.
#[derive(Debug, Clone, Copy, Default)]
pub struct HudInput {
    pub health: Option<Health>,
    pub weapon: Option<WeaponStatus>,
    pub respawn: Option<RespawnTimer>,
    /// The hit marker is running.
    pub hit: bool,
    /// Length of one tick.
    pub timestep: Duration,
}

/// What is shown: `None` (or false) hides the widget.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudView {
    pub crosshair: bool,
    pub ammo: Option<String>,
    pub hit: bool,
    pub health: Option<String>,
    pub respawn: Option<String>,
}

pub fn hud_view(input: HudInput) -> HudView {
    let alive = input.health.is_none_or(|health| health.is_alive());
    let weapon = input.weapon.filter(|_| alive);
    HudView {
        crosshair: weapon.is_some(),
        ammo: weapon.map(|weapon| {
            if weapon.reload_ticks_left > 0 {
                "Reloading".to_string()
            } else {
                format!("Ammo {}/{}", weapon.ammo, weapon.magazine)
            }
        }),
        hit: input.hit && input.weapon.is_some(),
        health: input
            .health
            .map(|health| format!("Health {}", health.current())),
        respawn: input.respawn.map(|timer| {
            // Whole seconds, rounded up: "1" until the respawn happens.
            let seconds = (input.timestep.as_secs_f64() * f64::from(timer.ticks_left))
                .ceil()
                .max(1.0);
            format!("Respawn in {seconds}")
        }),
    }
}
