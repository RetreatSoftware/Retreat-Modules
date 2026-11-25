# health (built)

Rules: `crates/retreat_module_health/src/lib.rs`. Numbers: `HealthConfig::default()` in `src/config.rs`.
Commands: `health.add_dummy <player>`, `health.damage <player> <amount> [attacker]`, `health.kill <player>`,
`assert.health`, `assert.health_dead` (none client allowed).

Still to do:
- Spawn protection should also end when the player fires: add a shared "player fired" message with `rifle`.
- `character_movement` teleports real characters on `PlayerRespawned` (needs C4); only health dummies
  move today.
- `player_spawn` shares the spawn choice (`choose_spawn_point`): move it to the contract then.
