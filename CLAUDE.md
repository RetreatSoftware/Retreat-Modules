# Retreat-Modules

A collection of reusable, pluggable gameplay systems (inventory, combat, equipment, crafting, progression, and others), designed to integrate with Retreat's engines and the Forge builder.

This file contains project facts and rules only. Personal preferences belong in `CLAUDE.local.md`.

The root `../CLAUDE.md` also applies here. This file may narrow its scope (see "Status") but not relax its rules; its verification and infrastructure rules apply only once this repository has code that needs them.

## Status

Early stage. Modules: `crates/retreat_module_spawner` (`spawner.rain <count>`, action `rain` on key R), the first module and the template for the next ones; `crates/retreat_module_orbit_camera` (`orbit.start|stop|set_angle`, held actions `orbit_left` / `orbit_right` on the arrow keys), which proves input + camera together (it edits a `GameCamera` and reads `InputActions::is_held` from a `FixedUpdate` system ordered `.after(InputActionSystems)`); `crates/retreat_module_sfx` (`sfx.play <path>`, `sfx.music <path>|off`, action `sfx_beep` on key P), which only spawns the contract's `SoundPlayback` and leaves playback to the engine; `crates/retreat_module_fox_walk` (`fox.set_speed <model> <speed>`), which only writes the contract's `AnimationPlayback` (pure `clips_for_speed` picks and blends Survey/Walk/Run); `crates/retreat_module_highscore` (`score.add|reset|save|load|show`), which saves a versioned document through `SaveStore` and migrates older data (pure `migrate`). A module that saves picks its `MODULE_ID` (letters, digits, `_`, `-`), owns its data `version`, builds JSON with `retreat_engine_api::serde_json` and never touches files or browser storage. Do not implement anything beyond what is explicitly requested.

## Layout and decisions

- **What is a module** (FRAMEWORK.md §12, engine/module split): gameplay that is a design choice differing between games, built on engine capabilities. Camera controllers, character controllers, which sound or animation plays when, menus and what gets saved are modules; audio/animation playback, input mapping, storage, UI toolkit, physics simulation and networking are engine capabilities. A module never contains platform-specific (`cfg(target_arch = "wasm32")`) code. When a module needs a capability the contract crate lacks, add a library-neutral entry to `retreat_engine_api` in Retreat-Engine-3D first.
- **One Cargo workspace** (`Cargo.toml` at the root), one crate per module under `crates/`, named `retreat_module_<id>`.
- **Dependencies:** only `bevy` (the exact version the engine pins, `0.20.1`) and the engine's contract crate `retreat_engine_api`, as a git dependency on the public `Retreat-Engine-3D` repository. `Cargo.lock` pins the commit; `cargo update -p retreat_engine_api` takes a newer one. Never depend on `retreat_engine` (engine internals), on a physics library, or on another module.
- **A module is a Bevy plugin** (`<Name>ModulePlugin`) that registers its commands with `AppCommandExt::register_command`. Every user-visible action is a command (`area.verb`, area = module id). Randomness comes only from `SimulationRng`. Physics objects are `retreat_engine_api::PhysicsBody` descriptions on named root entities (the engine creates the real bodies in a deterministic order).
- **Manifest:** `[package.metadata.retreat]` in each module's `Cargo.toml`, read later by Retreat-Forge. Provisional schema: `id`, `display_name`, `summary`, `plugin` (the plugin type path), `commands` (ids it registers), `requires` (engine capabilities, e.g. `physics`, `input`), `actions` (input action ids it registers; unique across modules, declared with `AppInputExt::register_action`), `engine_api` (compatible `retreat_engine_api` semver requirement).
- **Tests use only Bevy and the contract crate** (`MinimalPlugins` + `CommandRegistry` + `SimulationRng` + the plugin, run commands with `run_command_line`). The rendered and simulated result is checked in the engine sandbox, which adds the module (`Retreat-Engine-3D/scripts/verify/module_<id>.cmds`).
- **CI** (`.github/workflows/ci.yml`): fmt, clippy with warnings as errors, tests, and a `wasm32-unknown-unknown` check.

## Ecosystem context

Sibling repositories (cloned next to this one under `Retreat/`): `Retreat-CoreSDK`, `Retreat-Engine-3D`, `Retreat-Forge`, `Retreat-Website`. Vision documents live in the `Retreat` repository: [FRAMEWORK.md](https://github.com/RetreatSoftware/Retreat/blob/main/FRAMEWORK.md) and [PLATFORM.md](https://github.com/RetreatSoftware/Retreat/blob/main/PLATFORM.md).

## Rules

- Each module is an independent crate with a manifest (capabilities, dependencies, compatibility, configuration). Modules depend on `Retreat-CoreSDK` and the engine's public API / contract crates, never on each other and never on engine internals.
- **Shared generic helpers:** check `Retreat-CoreSDK` first. A helper stays inside the module that first needs it; when a second module needs it, move it to CoreSDK instead of copying it. What belongs in CoreSDK, and what does not, is defined in `../Retreat-CoreSDK/CLAUDE.md`. Shared gameplay concepts (items, damage, health) go in contract crates, not in CoreSDK and not duplicated across modules.
- Modules must build for both native and `wasm32-unknown-unknown`.

## Working rules

- Keep the initial implementation as simple as reasonably possible.
- Do not modify sibling repositories from this one without being asked.
- Write documentation and code comments in English.
