//! Retreat module manifest v1 (pure library, no Bevy; builds for native and wasm32).
//!
//! A module's manifest is the `[package.metadata.retreat]` table of its `Cargo.toml`. This crate
//! owns the schema (`Manifest`), the engine capability list (`capabilities`), the checks of a
//! manifest against the module's source (`source_scan`, `validate`) and the checks across all
//! modules of a workspace (`workspace`). Retreat-Forge and `retreat_resolver` read manifests only
//! through this crate. The schema is documented in `Retreat-Modules/CLAUDE.md`.

pub mod capabilities;
pub mod manifest;
pub mod problem;
pub mod source_scan;
pub mod validate;
pub mod workspace;

pub use manifest::{MANIFEST_VERSION, Manifest, NetworkRole, Target};
pub use problem::Problem;
