# Retreat-Modules

A collection of reusable, pluggable gameplay systems — such as inventory, combat, equipment, crafting, progression, and other game features.

Modules are independent and combinable, and are designed to integrate with Retreat's engines and the Retreat-Forge builder.

## Part of the Retreat ecosystem

- [Retreat-CoreSDK](https://github.com/RetreatSoftware/Retreat-CoreSDK) — shared SDK
- [Retreat-Website](https://github.com/RetreatSoftware/Retreat-Website) — web platform
- [Retreat-Forge](https://github.com/RetreatSoftware/Retreat-Forge) — game creation tool
- [Retreat-Engine-3D](https://github.com/RetreatSoftware/Retreat-Engine-3D) — 3D game engine

## Modules

| Crate | Commands | What it does |
|---|---|---|
| `retreat_module_spawner` | `spawner.rain <count>` | Drops physics boxes from the sky (first, demo module). |
| `retreat_module_arena` | `arena.load <map>` | Builds an arena map from fixed physics boxes and publishes spawn points. |
| `retreat_module_health` | `health.damage`, `health.kill`, `health.add_dummy` | Damage, death and respawn at spawn points. |
| `retreat_module_match` | `match.start`, `match.end`, `match.restart` | Timed match with scoreboard, winner and restart. |
| `retreat_module_player_spawn` | `spawn.join`, `spawn.solo`, `spawn.leave` | A character at a spawn point per player. |
| `retreat_module_character_movement` | `movement.input`, `movement.look` | Walk, sprint, jump, dash, mouse look. |
| `retreat_module_third_person_camera` | `shoulder.follow`, `shoulder.swap` | Over-the-shoulder camera that stays out of walls. |

Shared gameplay types live in the contract crate `retreat_gameplay_api`. Planned modules: [docs/modules](docs/modules/README.md).

```bash
cargo test --workspace
cargo check --workspace --target wasm32-unknown-unknown
```

The engine sandbox (Retreat-Engine-3D) adds these modules; see its `scripts/verify/module_*.cmds`.
