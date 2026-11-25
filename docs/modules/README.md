# Planned arena modules

Design notes for the modules of `ARENA_PLAN.md` (Retreat repository). `player_spawn`, `character_movement`, `third_person_camera`, `character_visuals`, `health` and `match` are
built.
Defaults decided 2026-11-25. When a module is built, its `<Name>Config::default()` becomes the
source of its numbers: replace the note's table with a pointer to the code.

| Note | Module id | Command area | Waits for (engine) | Plan step |
|---|---|---|---|---|
| [player_spawn](player_spawn.md) | `player_spawn` | `spawn.` | built | 1, 2 |
| [character_movement](character_movement.md) | `character_movement` | `movement.` | built | 1 |
| [third_person_camera](third_person_camera.md) | `third_person_camera` | `shoulder.` | built | 1 |
| [health](health.md) | `health` | `health.` | built | 3 |
| [rifle](rifle.md) | `rifle` | `rifle.` | C1, C2, C3, C7, C9 | 3 |
| [projectile_launcher](projectile_launcher.md) | `projectile_launcher` | `launcher.` | C1, C2, C3, C8 | 4 |
| [match](match.md) | `match` | `match.` | built | 5 |
| [combat_hud](combat_hud.md) | `combat_hud` | `hud.` | C9 | 3, 5 |
| [character_visuals](character_visuals.md) | `character_visuals` | `visuals.` | built | 2 |

Already built: `arena` (`arena.load <map>`, maps and spawn points).

## Engine capabilities (each needs a library-neutral entry in `retreat_engine_api`)

| Code | Capability |
|---|---|
| C1 | Rollback and replication of module and contract state (done: `NetState` components) |
| C2 | Mouse look, analog values, cursor capture (done) |
| C3 | Ray and shape casts against bodies (done) |
| C4 | Upright character bodies, step and slope handling, teleport (done) |
| C5 | Smoothing of remote players (done) |
| C6 | Per-entity color and character models set from a module (done) |
| C7 | Lag compensation: query the world as the shooter saw it |
| C8 | Spawn and replicate non-box entities (projectiles), predicted on the shooter's client |
| C9 | Changing on-screen text and a crosshair |

## Rules for all of them

- Modules share only `retreat_gameplay_api` types; each works when the others are absent.
- The server decides damage, death, respawn, ammo and score. Clients predict movement, own shots
  and own projectiles.
- Time in config is in seconds; a module converts to fixed ticks once (rounding up) and stores ticks.
- Tuning is a `<Name>Config` resource (`Default` below), overridden by the game before the plugin is added.
- `assert.*` commands needed by scripts are added with the module that owns the state.
