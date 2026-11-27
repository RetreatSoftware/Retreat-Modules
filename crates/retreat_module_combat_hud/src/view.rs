//! What the HUD shows for a given state. Pure.

use retreat_gameplay_api::{
    Health, Inventory, ItemCatalog, ItemKind, ItemStack, RespawnTimer, WeaponKind, WeaponStatus,
    Weapons, owns_weapon, reserve_of,
};
use std::time::Duration;

/// What this machine's player has; each part is absent when no module provides it.
#[derive(Debug, Clone, Default)]
pub struct HudInput {
    pub health: Option<Health>,
    pub weapons: Option<Weapons>,
    pub inventory: Option<Inventory>,
    /// What the running pickup notice names.
    pub taken: Vec<ItemStack>,
    pub catalog: ItemCatalog,
    pub respawn: Option<RespawnTimer>,
    /// The hit marker is running.
    pub hit: bool,
    /// The scoreboard covers the middle of the screen: nothing else is shown there.
    pub covered: bool,
    /// Length of one tick.
    pub timestep: Duration,
}

/// What is shown: `None` (or false) hides the widget.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudView {
    pub crosshair: bool,
    /// One line per carried weapon, in kind order; empty hides them all.
    pub ammo: Vec<(WeaponKind, String)>,
    /// One line per carried item to use, in catalog order.
    pub items: Vec<String>,
    /// What was just picked up ([`pickup_lines`]); empty when nothing was.
    pub pickups: Vec<String>,
    pub hit: bool,
    pub health: Option<String>,
    pub respawn: Option<String>,
}

pub fn hud_view(input: HudInput) -> HudView {
    let alive = input.health.is_none_or(|health| health.is_alive());
    let (inventory, catalog) = (input.inventory.as_ref(), &input.catalog);
    let carried: Vec<&WeaponStatus> = input
        .weapons
        .iter()
        .flat_map(Weapons::iter)
        .filter(|weapon| owns_weapon(inventory, catalog, weapon.kind))
        .collect();
    let armed = !carried.is_empty();
    let usable = armed && alive;
    // Ammunition is read with its weapon and weapons by their ammunition: the lines of their own
    // are the items to use.
    let items = catalog
        .0
        .iter()
        .filter(|item| matches!(item.kind, ItemKind::Heal(_)) && alive)
        .filter_map(|item| {
            let count = inventory?.count(&item.id);
            (count > 0).then(|| format!("{} x{count}", item.name))
        })
        .collect();
    HudView {
        crosshair: usable && !input.covered,
        ammo: carried
            .iter()
            .filter(|_| alive)
            .map(|weapon| {
                let reserve = reserve_of(inventory, catalog, weapon.kind);
                (weapon.kind, weapon_text(weapon, reserve))
            })
            .collect(),
        items,
        pickups: if alive && !input.covered {
            pickup_lines(&input.taken, catalog)
        } else {
            Vec::new()
        },
        hit: input.hit && armed && !input.covered,
        health: input
            .health
            .map(|health| format!("Health {}", health.current())),
        respawn: input.respawn.filter(|_| !input.covered).map(|timer| {
            // Whole seconds, rounded up: "1" until the respawn happens.
            let seconds = (input.timestep.as_secs_f64() * f64::from(timer.ticks_left))
                .ceil()
                .max(1.0);
            format!("Respawn in {seconds}")
        }),
    }
}

/// One weapon's name and ammo: the magazine, and the reserve when it is limited.
fn weapon_text(weapon: &WeaponStatus, reserve: Option<u32>) -> String {
    if weapon.reload_ticks_left > 0 {
        format!("{} reloading", weapon.kind.label())
    } else {
        let reserve = reserve.map_or_else(String::new, |rounds| format!(" +{rounds}"));
        format!(
            "{} {}/{}{reserve}",
            weapon.kind.label(),
            weapon.ammo,
            weapon.magazine
        )
    }
}

/// What was just picked up, one line per item: `+30 Rifle rounds`. Amounts of one item are summed.
pub fn pickup_lines(taken: &[ItemStack], catalog: &ItemCatalog) -> Vec<String> {
    catalog
        .0
        .iter()
        .filter_map(|item| {
            let count: u32 = taken
                .iter()
                .filter(|stack| stack.item == item.id)
                .map(|stack| stack.count)
                .sum();
            (count > 0).then(|| format!("+{count} {}", item.name))
        })
        .collect()
}
