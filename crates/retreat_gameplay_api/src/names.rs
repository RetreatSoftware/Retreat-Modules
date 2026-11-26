//! What players are called on screen.

use crate::PlayerId;
use bevy::prelude::*;

/// The names a game gives its players (for example their colors): player `id` is
/// `names[id % names.len()]`. Empty (the default) numbers them. Not shared state: the game inserts
/// the same value on every machine.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerNames(pub Vec<String>);

impl PlayerNames {
    pub fn of(&self, player: PlayerId) -> String {
        match self.0.len() {
            0 => format!("Player {}", player.0),
            count => self.0[player.0 as usize % count].clone(),
        }
    }
}
