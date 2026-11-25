//! The score of the running match.

use crate::PlayerId;
use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// One player's line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub player: PlayerId,
    /// What ranks players; may be negative.
    pub points: i32,
    pub kills: u32,
    pub deaths: u32,
}

impl ScoreEntry {
    pub fn new(player: PlayerId) -> Self {
        Self {
            player,
            points: 0,
            kills: 0,
            deaths: 0,
        }
    }
}

/// Every player's score: a component of the match entity ([`crate::MATCH_ENTITY_NAME`]). Owned by
/// the match module; others only read it.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scoreboard {
    /// Sorted by player id, one entry per player.
    entries: Vec<ScoreEntry>,
}

impl NetState for Scoreboard {
    const ID: &'static str = "gameplay.scoreboard";
}

impl Scoreboard {
    /// The entry of `player`, created with zeros if missing.
    pub fn entry_mut(&mut self, player: PlayerId) -> &mut ScoreEntry {
        let index = match self.entries.binary_search_by_key(&player, |e| e.player) {
            Ok(index) => index,
            Err(index) => {
                self.entries.insert(index, ScoreEntry::new(player));
                index
            }
        };
        &mut self.entries[index]
    }

    pub fn entry(&self, player: PlayerId) -> Option<&ScoreEntry> {
        self.entries.iter().find(|e| e.player == player)
    }

    pub fn remove(&mut self, player: PlayerId) {
        self.entries.retain(|e| e.player != player);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Display order: most points, then fewest deaths, then lowest player id.
    pub fn standings(&self) -> Vec<ScoreEntry> {
        let mut ranked = self.entries.clone();
        ranked.sort_by_key(|e| (std::cmp::Reverse(e.points), e.deaths, e.player));
        ranked
    }

    /// The only player with the most points; `None` when empty or tied for first (a draw).
    pub fn winner(&self) -> Option<PlayerId> {
        match self.standings().as_slice() {
            [] => None,
            [first, rest @ ..] => rest
                .first()
                .is_none_or(|second| second.points < first.points)
                .then_some(first.player),
        }
    }
}
