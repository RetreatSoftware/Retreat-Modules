//! What a player's weapons show: written by the weapon modules, read by the HUD.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// Which weapon. One variant per weapon module; the order is the order they are listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeaponKind {
    Rifle,
    Launcher,
}

impl WeaponKind {
    /// Every kind, in listing order (a HUD makes one line for each).
    pub const ALL: [WeaponKind; 2] = [WeaponKind::Rifle, WeaponKind::Launcher];

    /// The name a player reads.
    pub fn label(self) -> &'static str {
        match self {
            WeaponKind::Rifle => "Rifle",
            WeaponKind::Launcher => "Rockets",
        }
    }
}

/// One weapon a player carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponStatus {
    pub kind: WeaponKind,
    /// Shots left in the magazine.
    pub ammo: u32,
    pub magazine: u32,
    /// Ticks until a running reload is done; 0 when not reloading.
    pub reload_ticks_left: u32,
    /// Shots of this weapon that really hurt someone, as the server counts them. It only grows, so
    /// a reader shows a hit marker when it rises.
    pub hits_landed: u32,
}

/// On a player: every weapon it carries, at most one per [`WeaponKind`], sorted by kind (so every
/// machine has the same value whichever module equipped first). A weapon module changes only its
/// own entry.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weapons(Vec<WeaponStatus>);

impl NetState for Weapons {
    const ID: &'static str = "gameplay.weapons";
}

impl Weapons {
    pub fn of(&self, kind: WeaponKind) -> Option<&WeaponStatus> {
        self.0.iter().find(|weapon| weapon.kind == kind)
    }

    pub fn of_mut(&mut self, kind: WeaponKind) -> Option<&mut WeaponStatus> {
        self.0.iter_mut().find(|weapon| weapon.kind == kind)
    }

    /// Adds the weapon, or replaces the one of the same kind.
    pub fn put(&mut self, status: WeaponStatus) {
        match self.of_mut(status.kind) {
            Some(found) => *found = status,
            None => {
                self.0.push(status);
                self.0.sort_by_key(|weapon| weapon.kind);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &WeaponStatus> {
        self.0.iter()
    }
}
