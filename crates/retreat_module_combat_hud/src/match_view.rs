//! What the HUD shows of the match: clock, result, scoreboard and the kill feed. Pure.

use retreat_gameplay_api::{
    KillEntry, KillLog, MatchPhase, MatchState, PlayerId, PlayerRoster, Scoreboard,
};
use std::time::Duration;

/// The match as this machine knows it; each part is absent when no module provides it.
#[derive(Debug, Clone, Default)]
pub struct MatchInput {
    pub state: Option<MatchState>,
    pub board: Option<Scoreboard>,
    /// This machine's player, marked on the scoreboard.
    pub own: Option<PlayerId>,
    /// The scoreboard is asked for (button held or switched on).
    pub scoreboard_wanted: bool,
    /// Length of one tick.
    pub timestep: Duration,
}

/// What is shown: `None` or empty hides the widgets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchView {
    /// Time left, or the countdown to the next match.
    pub clock: Option<String>,
    pub result: Option<PlayerLine>,
    pub score_head: Option<String>,
    /// One line per player on the scoreboard, best first.
    pub score_rows: Vec<PlayerLine>,
}

/// A line about a player, shown in that player's color (`None`: about nobody).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLine {
    pub player: Option<PlayerId>,
    pub text: String,
}

pub fn match_view(input: &MatchInput, roster: &PlayerRoster) -> MatchView {
    // Whole seconds, rounded up: "1" until the moment itself.
    let seconds = |ticks: u32| {
        (input.timestep.as_secs_f64() * f64::from(ticks))
            .ceil()
            .max(1.0) as u64
    };
    let (clock, result, finished) = match input.state.map(|state| state.phase) {
        None => (None, None, false),
        Some(MatchPhase::Running { ticks_left }) => {
            let left = seconds(ticks_left);
            (Some(format!("{}:{:02}", left / 60, left % 60)), None, false)
        }
        Some(MatchPhase::Finished {
            winner,
            ticks_until_restart,
        }) => (
            Some(format!("Next match in {}", seconds(ticks_until_restart))),
            Some(PlayerLine {
                player: winner,
                text: winner.map_or_else(
                    || "Draw".to_string(),
                    |winner| format!("{} wins", roster.name(winner)),
                ),
            }),
            true,
        ),
    };
    let board = input
        .board
        .as_ref()
        .filter(|_| finished || input.scoreboard_wanted);
    MatchView {
        clock,
        result,
        score_head: board.map(|_| score_line("", "Player", "Pts", "K", "D")),
        score_rows: board.map_or_else(Vec::new, |board| {
            board
                .standings()
                .iter()
                .map(|line| {
                    let mark = if input.own == Some(line.player) {
                        ">"
                    } else {
                        ""
                    };
                    PlayerLine {
                        player: Some(line.player),
                        text: score_line(
                            mark,
                            roster.name(line.player),
                            &line.points.to_string(),
                            &line.kills.to_string(),
                            &line.deaths.to_string(),
                        ),
                    }
                })
                .collect()
        }),
    }
}

/// Columns of fixed width: the HUD's font has one width for every character.
fn score_line(mark: &str, name: &str, points: &str, kills: &str, deaths: &str) -> String {
    format!("{mark:<2}{name:<9.9}{points:>4}{kills:>4}{deaths:>4}")
}

/// One line of the kill feed, in the killer's color (the victim's without one).
pub fn kill_line(entry: &KillEntry, roster: &PlayerRoster) -> PlayerLine {
    let victim = roster.name(entry.victim);
    let text = match entry.killer {
        Some(killer) if killer != entry.victim => {
            format!("{} killed {victim}", roster.name(killer))
        }
        Some(_) => format!("{victim} blew up"),
        None => format!("{victim} fell"),
    };
    PlayerLine {
        player: Some(entry.killer.unwrap_or(entry.victim)),
        text,
    }
}

/// The deaths this machine is showing. Not shared state: each machine times its own.
#[derive(Debug, Clone, Default)]
pub struct KillFeed {
    /// The log's next number when last seen; `None` before there was a log.
    seen: Option<u32>,
    /// Oldest first, with the ticks each still stays.
    shown: Vec<(KillEntry, u32)>,
}

impl KillFeed {
    /// One tick: older lines age, deaths that are new in `log` appear for `lifetime` ticks. The
    /// first log seen shows nothing (a player who joins late missed those deaths).
    pub fn tick(&mut self, log: Option<&KillLog>, lifetime: u32) {
        for (_, ticks_left) in &mut self.shown {
            *ticks_left = ticks_left.saturating_sub(1);
        }
        self.shown.retain(|(_, ticks_left)| *ticks_left > 0);
        let Some(log) = log else {
            self.seen = None;
            self.shown.clear();
            return;
        };
        let next = log.next_number();
        // A correction can take back a predicted death.
        self.shown.retain(|(entry, _)| entry.number < next);
        if let Some(seen) = self.seen {
            self.shown.extend(
                log.entries()
                    .iter()
                    .filter(|entry| entry.number >= seen)
                    .map(|entry| (*entry, lifetime)),
            );
        }
        self.seen = Some(next);
    }

    /// The latest `most` deaths, oldest first.
    pub fn latest(&self, most: usize) -> impl Iterator<Item = &KillEntry> {
        let skipped = self.shown.len().saturating_sub(most);
        self.shown.iter().skip(skipped).map(|(entry, _)| entry)
    }
}
