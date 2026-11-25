//! The scoring rules, without a Bevy app.

use crate::config::MatchConfig;
use retreat_gameplay_api::{PlayerDied, PlayerId, Scoreboard};

/// A board with a zero line for each of `players`.
pub fn scoreboard_for(players: &[PlayerId]) -> Scoreboard {
    let mut board = Scoreboard::default();
    for &player in players {
        board.entry_mut(player);
    }
    board
}

/// Counts one death. Players without a line (they left) neither lose nor gain.
pub fn score_death(board: &mut Scoreboard, death: PlayerDied, config: &MatchConfig) {
    let killer = death.killer.filter(|&killer| killer != death.victim);
    if board.entry(death.victim).is_some() {
        let victim = board.entry_mut(death.victim);
        victim.deaths += 1;
        if killer.is_none() {
            victim.points += config.points_per_self_kill;
        }
    }
    if let Some(killer) = killer
        && board.entry(killer).is_some()
    {
        let entry = board.entry_mut(killer);
        entry.kills += 1;
        entry.points += config.points_per_kill;
    }
}
