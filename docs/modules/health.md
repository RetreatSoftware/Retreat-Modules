# health (built)

Rules: `crates/retreat_module_health/src/lib.rs`. Numbers: `HealthConfig::default()` in `src/config.rs`.
Commands: `health.add_dummy <player>`, `health.damage <player> <amount> [attacker]`, `health.kill <player>`,
`assert.health`, `assert.health_dead` (none client allowed).

What death means elsewhere: `character_movement` does not steer the dead, `rifle` neither lets them shoot
nor hits them, `character_visuals` hides them. Their body stays where it is until the respawn, so it can
still block others.
