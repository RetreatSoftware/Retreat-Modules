# player_spawn (replaces the box movement of `players` in the arena preset)

One character per joined client, at a spawn point; removed when the client leaves. `players` stays
for the sandbox preset until the arena preset replaces it; a game never adds both.

- **Commands:** `spawn.join <id>` and `spawn.leave <id>` (single player and scripts; the same code the
  network messages run), `assert.spawn_count <n>`.
- **Actions:** none.
- **Reads:** `NetClientJoined`, `NetClientLeft`, `SpawnPoints`, `MatchState`.
- **Writes:** an entity `player_<id>` with `PlayerId` and a character body; `RemovePhysicsBody` on leave.
- **Config:** `max_players` 8 (`MAX_PLAYERS`); a join beyond it is refused with a clear log.
- **Spawn choice:** farthest from the nearest living player, ties by point index.
- **Without a map:** a join fails with "no spawn points; load a map first".
- **Waits for:** C1, C4 (character body), C6 only for looks.
