# player_spawn (built)

Rules: `crates/retreat_module_player_spawn/src/lib.rs`. Commands: `spawn.join <player>`, `spawn.solo <player>`,
`spawn.leave <player>`, `assert.spawn_count <n>` (none client allowed). At most `MAX_PLAYERS`.
Replaces the box movement of `players` in the arena; a game never adds both.
Crawler mode (`PlayerSpawnConfig::spawn_on_join: false`, 2026-11-27): bodies of saved characters appear on
`CharacterEntered` at their saved place and are saved on leaving, on disconnect and every `autosave_seconds`
(`src/characters.rs`).
