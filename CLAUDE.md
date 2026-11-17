# Retreat-Modules

A collection of reusable, pluggable gameplay systems (inventory, combat, equipment, crafting, progression, and others), designed to integrate with Retreat's engines and the Forge builder.

This file contains project facts and rules only. Personal preferences belong in `CLAUDE.local.md`.

The root `../CLAUDE.md` also applies here. This file may narrow its scope (see "Status") but not relax its rules; its verification and infrastructure rules apply only once this repository has code that needs them.

## Status

Early stage. No code exists yet. Do not implement anything beyond what is explicitly requested.

## Ecosystem context

Sibling repositories (cloned next to this one under `Retreat/`): `Retreat-CoreSDK`, `Retreat-Engine-3D`, `Retreat-Forge`, `Retreat-Website`. Vision documents live in the parent folder: `FRAMEWORK.en.md` and `PLATFORM.en.md`.

## Rules

- Each module is an independent crate with a manifest (capabilities, dependencies, compatibility, configuration). Modules depend on `Retreat-CoreSDK` and the engine's public API / contract crates, never on each other and never on engine internals.
- **Shared generic helpers:** check `Retreat-CoreSDK` first. A helper stays inside the module that first needs it; when a second module needs it, move it to CoreSDK instead of copying it. What belongs in CoreSDK, and what does not, is defined in `../Retreat-CoreSDK/CLAUDE.md`. Shared gameplay concepts (items, damage, health) go in contract crates, not in CoreSDK and not duplicated across modules.
- Modules must build for both native and `wasm32-unknown-unknown`.

## Working rules

- Keep the initial implementation as simple as reasonably possible.
- Do not modify sibling repositories from this one without being asked.
- Write documentation and code comments in English.
