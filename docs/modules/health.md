# health (built)

Rules: `crates/retreat_module_health/src/lib.rs`. Numbers: `HealthConfig::default()` in `src/config.rs`.
Commands: `health.add_dummy <player>`, `health.damage <player> <amount> [attacker]`, `health.kill <player>`,
`assert.health`, `assert.health_dead` (none client allowed).

What death means elsewhere: this module makes the dead body `Intangible` (walked through, ignored by
casts), `character_movement` does not steer the dead, weapons do not let them shoot, and
`character_visuals` hides them.
