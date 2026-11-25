# match

Timed free-for-all: score, winner, restart.

- **Commands:** `match.start`, `match.end`, `match.restart` (server and scripts, never client allowed),
  `assert.match_running`, `assert.match_finished`, `assert.match_score <player> <points>`.
- **Actions:** none.
- **Reads:** `PlayerDied`, `PlayerId` added and removed (scoreboard lines).
- **Writes:** `Scoreboard`, `MatchState`, `MatchEnded`, `MatchRestarted`.
- **Config (default):** match length 300 s; result screen 10 s, then automatic restart; start when the
  first player joins; kill +1; self-kill or world death -1; no score limit; a tie for first is a draw
  (`Scoreboard::winner`).
- **Without health:** no deaths arrive; the match still runs its timer.
- **Waits for:** C1.
