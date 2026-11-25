# combat_hud

Crosshair, own health, timer, scoreboard, result. Client only; reads, never changes game state.

- **Commands:** `hud.scoreboard <on|off>`, `assert.hud_text <element> <text>` (text without spaces).
- **Actions:** `hud_scoreboard` Tab (held shows it).
- **Reads:** `Health`, `Scoreboard`, `MatchState`, `DamageApplied` (hit marker, damage flash),
  `PlayerDied` (kill feed). Each element is hidden when its data is absent (no match: no timer);
  the layout is computed from what is shown by a pure function.
- **Writes:** on-screen elements only. Ammo is not shown until a shared ammo type exists in the contract
  (added with `rifle`).
- **Config (default):** hit marker 0.1 s, kill feed 5 lines for 4 s each.
- **Waits for:** C9.
