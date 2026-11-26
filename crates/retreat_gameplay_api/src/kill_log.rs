//! Who killed whom lately, as shared state: messages of the server do not reach clients.

use crate::{MAX_PLAYERS, PlayerId};
use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// One death.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillEntry {
    /// Counts up over the whole session, so a reader knows which entries are new.
    pub number: u32,
    pub victim: PlayerId,
    /// `None` for the world, equal to `victim` for a self-kill.
    pub killer: Option<PlayerId>,
}

/// The latest deaths of the running match, oldest first: a component of the match entity. Owned by
/// the match module; others only read it. It keeps [`MAX_PLAYERS`] entries, so every player can
/// die in one tick without a death being dropped before anyone saw it.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillLog {
    next: u32,
    entries: Vec<KillEntry>,
}

impl NetState for KillLog {
    const ID: &'static str = "gameplay.kill_log";
}

impl KillLog {
    pub fn push(&mut self, victim: PlayerId, killer: Option<PlayerId>) {
        self.entries.push(KillEntry {
            number: self.next,
            victim,
            killer,
        });
        self.next += 1;
        if self.entries.len() > MAX_PLAYERS {
            self.entries.remove(0);
        }
    }

    /// Forgets the deaths; the numbers go on.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The number the next death gets.
    pub fn next_number(&self) -> u32 {
        self.next
    }

    pub fn entries(&self) -> &[KillEntry] {
        &self.entries
    }
}
