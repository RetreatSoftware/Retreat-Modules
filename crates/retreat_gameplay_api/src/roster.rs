//! Who players are on screen: a name and a color each, so a character and its HUD lines match.

use crate::PlayerId;
use bevy::prelude::*;

/// One player's name and color.
#[derive(Debug, Clone, PartialEq)]
pub struct RosterEntry {
    pub name: String,
    pub color: Color,
}

impl RosterEntry {
    pub fn new(name: &str, color: Color) -> Self {
        Self {
            name: name.to_string(),
            color,
        }
    }
}

/// The game's players: player `id` is entry `id % len`. The default is eight color names in their
/// colors; a game inserts its own before adding any module, the same value on every machine. Not
/// shared state.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PlayerRoster(pub Vec<RosterEntry>);

impl Default for PlayerRoster {
    fn default() -> Self {
        let entry = |name, red, green, blue| RosterEntry::new(name, Color::srgb(red, green, blue));
        Self(vec![
            entry("Red", 0.95, 0.35, 0.3),
            entry("Blue", 0.35, 0.6, 1.0),
            entry("Green", 0.4, 0.85, 0.4),
            entry("Yellow", 1.0, 0.85, 0.25),
            entry("Purple", 0.75, 0.45, 0.95),
            entry("Orange", 1.0, 0.6, 0.15),
            entry("Cyan", 0.3, 0.9, 0.9),
            entry("White", 0.95, 0.95, 0.95),
        ])
    }
}

impl PlayerRoster {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if self.0.is_empty() {
            return Err("the roster needs at least one entry".to_string());
        }
        Ok(())
    }

    /// Where `player`'s entry is in the list.
    pub fn index_of(&self, player: PlayerId) -> usize {
        player.0 as usize % self.0.len()
    }

    pub fn name(&self, player: PlayerId) -> &str {
        &self.0[self.index_of(player)].name
    }

    pub fn color(&self, player: PlayerId) -> Color {
        self.0[self.index_of(player)].color
    }
}
