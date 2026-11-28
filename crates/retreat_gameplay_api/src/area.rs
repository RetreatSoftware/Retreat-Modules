//! Areas (crawler plan, phase 1): the city and the dungeons are areas of one map, placed far apart
//! in one physics world for now. An area is map data with rules; a character is in exactly one
//! area ([`InArea`]). This is the seam where areas later move to servers of their own.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

use crate::SpawnPoint;

/// An area of the map (`city`, `dungeon_1`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AreaId(pub String);

/// What is allowed in an area. Data of the map, never code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaRules {
    /// Nobody can be hurt here (the city).
    pub safe: bool,
    /// Players may hurt players here. Never true in a safe area.
    pub pvp: bool,
    /// The difficulty tier (0 for the city).
    pub tier: u32,
    /// Characters above this level may not enter; `None`: no limit.
    pub max_entry_level: Option<u32>,
}

/// One area: a horizontal rectangle of the map (any height) with its rules.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub id: AreaId,
    /// The name players see (`City`).
    pub name: String,
    pub rules: AreaRules,
    /// Smallest x and z inside.
    pub min: Vec2,
    /// Largest x and z inside.
    pub max: Vec2,
    /// New characters start here (exactly one area of a map is the home).
    pub home: bool,
}

impl Area {
    /// True when `position` (any height) is inside the rectangle.
    pub fn contains(&self, position: Vec3) -> bool {
        (self.min.x..=self.max.x).contains(&position.x)
            && (self.min.y..=self.max.y).contains(&position.z)
    }
}

/// The loaded map's areas (empty for a map without areas, such as the arena). Filled by the map
/// module.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MapAreas(pub Vec<Area>);

impl MapAreas {
    /// The area containing `position`, if any.
    pub fn at(&self, position: Vec3) -> Option<&Area> {
        self.0.iter().find(|area| area.contains(position))
    }

    /// The area new characters start in.
    pub fn home(&self) -> Option<&Area> {
        self.0.iter().find(|area| area.home)
    }

    /// The area called `id`.
    pub fn get(&self, id: &str) -> Option<&Area> {
        self.0.iter().find(|area| area.id.0 == id)
    }

    /// The spawn points that lie in `area`, in their order.
    pub fn spawn_points_in(points: &[SpawnPoint], area: &Area) -> Vec<SpawnPoint> {
        points
            .iter()
            .filter(|point| area.contains(point.feet))
            .copied()
            .collect()
    }

    /// Every problem of the areas, each saying what to fix (empty: fine). Ids are one word of
    /// lowercase letters, digits and `_`, used once; rectangles are not empty and do not overlap;
    /// exactly one area is the home; a safe area allows no PvP.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.0.is_empty() {
            return problems;
        }
        let homes = self.0.iter().filter(|area| area.home).count();
        if homes != 1 {
            problems.push(format!("{homes} home areas, exactly 1 is needed"));
        }
        for (index, area) in self.0.iter().enumerate() {
            let id = &area.id.0;
            let valid_id = !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !valid_id {
                problems.push(format!(
                    "area id `{id}` must be lowercase letters, digits and `_`"
                ));
            }
            if self.0[..index].iter().any(|other| other.id == area.id) {
                problems.push(format!("area id `{id}` is used twice"));
            }
            if !(area.min.is_finite() && area.max.is_finite() && area.min.cmplt(area.max).all()) {
                problems.push(format!(
                    "area `{id}` needs finite corners with min below max"
                ));
            }
            if area.rules.safe && area.rules.pvp {
                problems.push(format!("area `{id}` is safe, so it cannot allow PvP"));
            }
            for other in &self.0[..index] {
                let overlap = area.max.min(other.max) - area.min.max(other.min);
                if overlap.min_element() > 0.0 {
                    problems.push(format!("areas `{}` and `{id}` overlap", other.id.0));
                }
            }
        }
        problems
    }
}

/// The area a character is in (on the character; shared state). Kept by the `areas` module.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InArea(pub AreaId);

impl NetState for InArea {
    const ID: &'static str = "gameplay.in_area";
}
