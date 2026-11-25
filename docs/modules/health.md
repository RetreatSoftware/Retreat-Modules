# health (built)

Rules: `crates/retreat_module_health/src/lib.rs`. Numbers: `HealthConfig::default()` in `src/config.rs`.
Commands: `health.add_dummy <player>`, `health.damage <player> <amount> [attacker]`, `health.kill <player>`,
`assert.health`, `assert.health_dead` (none client allowed).

Still to do:
- Spawn protection should also end when the player fires: add a shared "player fired" message with `rifle`.
