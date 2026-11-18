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

```bash
cargo test --workspace
cargo check --workspace --target wasm32-unknown-unknown
```

The engine sandbox (Retreat-Engine-3D) adds these modules; see its `scripts/verify/module_*.cmds`.
