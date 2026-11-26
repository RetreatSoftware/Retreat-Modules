//! The module's tuning numbers.

use bevy::prelude::*;

/// A game changes these by inserting the resource before adding the plugin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct LauncherConfig {
    /// Hit points a direct hit removes (and the blast at its center).
    pub damage: u32,
    /// Hit points the blast removes at its edge.
    pub edge_damage: u32,
    /// How far from a body the explosion still hurts it, metres.
    pub blast_radius: f32,
    /// Share of the damage the shooter takes from its own rocket, percent (0 to 100).
    pub self_damage_percent: u32,
    /// Flight speed, metres per second.
    pub speed: f32,
    /// Size of the rocket, for its looks and for what it touches, metres.
    pub rocket_radius: f32,
    /// A rocket that hits nothing explodes after this long.
    pub lifetime_seconds: f32,
    /// Time between two rockets while the button is held.
    pub fire_interval_seconds: f32,
    /// Rockets in a full magazine; the reserve is unlimited.
    pub magazine: u32,
    /// Time a reload takes. It starts by itself when the magazine is empty.
    pub reload_seconds: f32,
    /// How far the aim looks for what the crosshair shows, metres.
    pub aim_range: f32,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            damage: 80,
            edge_damage: 20,
            blast_radius: 3.0,
            self_damage_percent: 50,
            speed: 25.0,
            rocket_radius: 0.15,
            lifetime_seconds: 4.0,
            fire_interval_seconds: 0.9,
            magazine: 4,
            reload_seconds: 2.0,
            aim_range: 100.0,
        }
    }
}

impl LauncherConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if self.magazine == 0 {
            return Err("magazine must be at least 1".to_string());
        }
        if self.edge_damage > self.damage {
            return Err("edge_damage must not be more than damage".to_string());
        }
        if self.self_damage_percent > 100 {
            return Err("self_damage_percent must be 0 to 100".to_string());
        }
        for (field, value) in [
            ("blast_radius", self.blast_radius),
            ("speed", self.speed),
            ("rocket_radius", self.rocket_radius),
            ("lifetime_seconds", self.lifetime_seconds),
            ("fire_interval_seconds", self.fire_interval_seconds),
            ("reload_seconds", self.reload_seconds),
            ("aim_range", self.aim_range),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("{field} must be more than 0, got {value}"));
            }
        }
        Ok(())
    }
}
