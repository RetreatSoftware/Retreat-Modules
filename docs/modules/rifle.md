# rifle (built)

Rules: `crates/retreat_module_rifle/src/lib.rs`. Numbers: `RifleConfig::default()` in `src/config.rs`.
Commands: `rifle.fire <yaw> <pitch> [view_tick]`, `rifle.reload` (both client allowed),
`assert.rifle_ammo`, `assert.rifle_hits`. Actions: `rifle_fire` (left mouse, held), `rifle_reload` (R).
Every shot shows a muzzle flash and a dotted tracer (engine effects, from the synced `RifleShot`).

Not built: sounds, spread, falloff, headshots. Needs `character_visuals` (or another module that
gives other players `RemoteInterpolation`) for shots to be rewound; without it they are judged against
the present.
