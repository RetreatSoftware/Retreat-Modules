# health

Damage, death, respawn. Server only decides.

- **Commands:** `health.damage <player> <amount>` and `health.kill <player>` (server and scripts, never
  client allowed; cause `World`), `assert.health <player> <hp>`, `assert.health_dead <player>`.
- **Actions:** none.
- **Reads:** `DamageRequested`, `PlayerId` (new ones get `Health`), `MatchState` (no damage after the end),
  `MatchRestarted` (everyone respawns), `SpawnPoints`.
- **Writes:** `Health`, `DamageApplied`, `PlayerDied`, `PlayerRespawned`.
- **Config (default):** max health 100, respawn delay 3 s, spawn protection 1 s (ends early when the
  player fires), no regeneration, fall-out height -10 m (kill with cause `World`).
- **Waits for:** C1 (health and timers replicate and roll back), C4 (teleport on respawn).
