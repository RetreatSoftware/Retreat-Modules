//! Creating the match and moving it through its phases.

use crate::config::MatchConfig;
use crate::rules::{score_death, scoreboard_for};
use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{
    MATCH_ENTITY_NAME, MatchEnded, MatchPhase, MatchRestarted, MatchState, PlayerDied, PlayerId,
    Scoreboard, ticks_from_seconds,
};

/// A fresh running phase.
pub(crate) fn running(config: &MatchConfig, time: &Time<Fixed>) -> MatchState {
    MatchState {
        phase: MatchPhase::Running {
            ticks_left: ticks_from_seconds(config.length_seconds, time.timestep()).max(1),
        },
    }
}

/// The finished phase with the winner `board` gives.
pub(crate) fn finished(board: &Scoreboard, config: &MatchConfig, time: &Time<Fixed>) -> MatchState {
    MatchState {
        phase: MatchPhase::Finished {
            winner: board.winner(),
            ticks_until_restart: ticks_from_seconds(config.result_seconds, time.timestep()).max(1),
        },
    }
}

/// What the match entity is spawned with.
pub(crate) fn new_match(
    players: &[PlayerId],
    config: &MatchConfig,
    time: &Time<Fixed>,
) -> (Name, MatchState, Scoreboard) {
    (
        Name::new(MATCH_ENTITY_NAME),
        running(config, time),
        scoreboard_for(players),
    )
}

pub(crate) fn start_with_first_player(
    matches: Query<(), (With<MatchState>, Without<RetiredEntity>)>,
    players: Query<&PlayerId, Without<RetiredEntity>>,
    config: Res<MatchConfig>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    if matches.is_empty() && !players.is_empty() {
        let present: Vec<PlayerId> = players.iter().copied().collect();
        commands.spawn(new_match(&present, &config, &time));
    }
}

pub(crate) fn score_and_advance(
    mut deaths: MessageReader<PlayerDied>,
    mut matches: Query<(&mut MatchState, &mut Scoreboard), Without<RetiredEntity>>,
    players: Query<&PlayerId, Without<RetiredEntity>>,
    config: Res<MatchConfig>,
    time: Res<Time<Fixed>>,
    mut ended: MessageWriter<MatchEnded>,
    mut restarted: MessageWriter<MatchRestarted>,
) {
    // Read every death, also while no match runs: a message never waits for a later tick.
    let deaths: Vec<PlayerDied> = deaths.read().copied().collect();
    let mut present: Vec<PlayerId> = players.iter().copied().collect();
    present.sort();
    for (mut state, mut board) in &mut matches {
        // One line per present player.
        let mut next = scoreboard_for(&present);
        for &player in &present {
            if let Some(line) = board.entry(player) {
                *next.entry_mut(player) = *line;
            }
        }
        match state.phase {
            MatchPhase::Running { ticks_left } => {
                for &death in &deaths {
                    score_death(&mut next, death, &config);
                }
                if ticks_left > 1 {
                    state.phase = MatchPhase::Running {
                        ticks_left: ticks_left - 1,
                    };
                } else {
                    *state = finished(&next, &config, &time);
                    ended.write(MatchEnded {
                        winner: next.winner(),
                    });
                }
            }
            MatchPhase::Finished {
                winner,
                ticks_until_restart,
            } => {
                if ticks_until_restart > 1 {
                    state.phase = MatchPhase::Finished {
                        winner,
                        ticks_until_restart: ticks_until_restart - 1,
                    };
                } else {
                    *state = running(&config, &time);
                    next = scoreboard_for(&present);
                    restarted.write(MatchRestarted);
                }
            }
        }
        board.set_if_neq(next);
    }
}
