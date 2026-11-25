//! The phase of the match.

use crate::PlayerId;
use bevy::prelude::*;

/// Where the match is. Times are fixed simulation ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MatchPhase {
    /// No match is being run (also the value in a game without a match module): everyone may play.
    #[default]
    NoMatch,
    Running {
        ticks_left: u32,
    },
    /// Showing the result; weapons do not fire and damage is ignored.
    Finished {
        winner: Option<PlayerId>,
        ticks_until_restart: u32,
    },
}

/// The current phase. Owned by the match module; others only read it.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchState {
    pub phase: MatchPhase,
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
