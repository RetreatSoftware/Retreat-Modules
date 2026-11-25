# player_spawn (built)

Rules: `crates/retreat_module_player_spawn/src/lib.rs`. Commands: `spawn.join <player>`, `spawn.solo <player>`,
`spawn.leave <player>`, `assert.spawn_count <n>` (none client allowed). At most `MAX_PLAYERS`.
Replaces the box movement of `players` in the arena; a game never adds both.
