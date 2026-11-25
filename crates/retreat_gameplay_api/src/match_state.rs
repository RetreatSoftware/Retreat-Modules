//! The phase of the match.

use crate::PlayerId;
use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

/// The entity name of the match: it carries [`MatchState`] and [`crate::Scoreboard`].
pub const MATCH_ENTITY_NAME: &str = "match";

/// Where the match is. Times are fixed simulation ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchPhase {
    Running {
        ticks_left: u32,
    },
    /// Showing the result; weapons do not fire and damage is ignored.
    Finished {
        winner: Option<PlayerId>,
        ticks_until_restart: u32,
    },
}

/// The current phase: a component of the match entity. Owned by the match module; others only read
/// it. A game without a match module has no such entity, and then everyone may fight.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    pub phase: MatchPhase,
}

impl NetState for MatchState {
    const ID: &'static str = "gameplay.match";
}

impl MatchState {
    /// False only while a finished match shows its result.
    pub fn allows_combat(&self) -> bool {
        !matches!(self.phase, MatchPhase::Finished { .. })
    }
}

/// The match just ended.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchEnded {
    pub winner: Option<PlayerId>,
}

/// A new match starts now: reset per-match state and respawn everyone.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchRestarted;
