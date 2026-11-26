//! What the HUD shows for a given state. Pure.

use retreat_gameplay_api::{Health, RespawnTimer, WeaponKind, WeaponStatus, Weapons};
use std::time::Duration;

/// What this machine's player has; each part is absent when no module provides it.
#[derive(Debug, Clone, Default)]
pub struct HudInput {
    pub health: Option<Health>,
    pub weapons: Option<Weapons>,
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
    /// One line per carried weapon, in kind order; empty hides them all.
    pub ammo: Vec<(WeaponKind, String)>,
    pub hit: bool,
    pub health: Option<String>,
    pub respawn: Option<String>,
}

pub fn hud_view(input: HudInput) -> HudView {
    let alive = input.health.is_none_or(|health| health.is_alive());
    let armed = input
        .weapons
        .as_ref()
        .filter(|weapons| weapons.iter().next().is_some());
    let usable = armed.filter(|_| alive);
    HudView {
        crosshair: usable.is_some(),
        ammo: usable.map_or_else(Vec::new, |weapons| {
            weapons
                .iter()
                .map(|weapon| (weapon.kind, weapon_text(weapon)))
                .collect()
        }),
        hit: input.hit && armed.is_some(),
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

/// One weapon's name and ammo.
fn weapon_text(weapon: &WeaponStatus) -> String {
    if weapon.reload_ticks_left > 0 {
        format!("{} reloading", weapon.kind.label())
    } else {
        format!(
            "{} {}/{}",
            weapon.kind.label(),
            weapon.ammo,
            weapon.magazine
        )
    }
}
