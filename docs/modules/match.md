# match (built)

Rules: `crates/retreat_module_match/src/lib.rs`. Numbers: `MatchConfig::default()` in `src/config.rs`.
Commands: `match.start`, `match.end`, `match.restart`, `match.set_clock <seconds>`,
`assert.match_running`, `assert.match_finished`, `assert.match_clock <least> <most>`,
`assert.match_score <player> <points>`, `assert.match_winner <player|draw>` (none client allowed).
Writes the contract's `MatchState`, `Scoreboard` and `KillLog` on the match entity.

Decided: no score limit; a leaver's score line is removed.
