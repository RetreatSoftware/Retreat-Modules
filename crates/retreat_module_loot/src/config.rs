//! What lies on the map, and the module's numbers.

use bevy::prelude::*;
use retreat_gameplay_api::{ItemCatalog, ItemId};

/// How a pickup is drawn: a glowing sphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickupLook {
    pub color: Color,
    /// Metres.
    pub radius: f32,
}

/// What one kind of spot holds.
#[derive(Debug, Clone, PartialEq)]
pub struct PickupKind {
    /// The map's name for the kind of spot (`health`).
    pub kind: String,
    /// What one taking gives, in this order.
    pub contents: Vec<(ItemId, u32)>,
    /// How long until it is there again.
    pub respawn_seconds: f32,
    pub look: PickupLook,
}

/// A game changes these by inserting the resource before adding the plugin. The default has no
/// pickups at all.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct LootConfig {
    pub kinds: Vec<PickupKind>,
    /// How near a player's feet must be, horizontally, metres.
    pub reach: f32,
    /// How long the dead's items lie there; 0 drops nothing.
    pub drop_seconds: f32,
    pub drop_look: PickupLook,
}

impl Default for LootConfig {
    fn default() -> Self {
        Self {
            kinds: Vec::new(),
            reach: 0.9,
            drop_seconds: 20.0,
            drop_look: PickupLook {
                color: Color::srgb(0.9, 0.9, 0.9),
                radius: 0.25,
            },
        }
    }
}

impl LootConfig {
    pub fn kind(&self, kind: &str) -> Option<&PickupKind> {
        self.kinds.iter().find(|found| found.kind == kind)
    }

    /// What is wrong, naming the field and the kind.
    pub fn validate(&self, catalog: &ItemCatalog) -> Result<(), String> {
        if !(self.reach.is_finite() && self.reach > 0.0) {
            return Err(format!("reach must be more than 0, got {}", self.reach));
        }
        if !(self.drop_seconds.is_finite() && self.drop_seconds >= 0.0) {
            return Err("drop_seconds must be 0 or more".to_string());
        }
        let mut looks = vec![("drop".to_string(), self.drop_look)];
        for (index, kind) in self.kinds.iter().enumerate() {
            let name = &kind.kind;
            if self.kinds[..index].iter().any(|other| other.kind == *name) {
                return Err(format!("kind `{name}` is listed twice"));
            }
            if !(kind.respawn_seconds.is_finite() && kind.respawn_seconds > 0.0) {
                return Err(format!(
                    "kind `{name}`: respawn_seconds must be more than 0"
                ));
            }
            if kind.contents.is_empty() {
                return Err(format!("kind `{name}` holds nothing"));
            }
            for (item, count) in &kind.contents {
                if catalog.get(item).is_none() {
                    return Err(format!("kind `{name}`: the catalog has no item `{item}`"));
                }
                if *count == 0 {
                    return Err(format!("kind `{name}`: the count of `{item}` is 0"));
                }
            }
            looks.push((name.clone(), kind.look));
        }
        match looks
            .iter()
            .find(|(_, look)| !(look.radius.is_finite() && look.radius > 0.0))
        {
            Some((name, _)) => Err(format!("kind `{name}`: the radius must be more than 0")),
            None => Ok(()),
        }
    }
}
