//! The engine capability list: the only names a manifest's `requires` may use.
//!
//! One source of truth. Decision: this list lives here until the engine's contract crate
//! (`retreat_engine_api`) publishes its own; then it moves there and this module re-exports it.
//! Adding a capability is a minor change of `CAPABILITY_LIST_VERSION`; removing or renaming one is
//! a major change of the engine contract (see "engine_api versioning" in Retreat-Modules/CLAUDE.md).

/// Version of this list. Bump when a name is added (and when removed, together with an
/// `engine_api` major bump).
pub const CAPABILITY_LIST_VERSION: u32 = 1;

/// Every engine capability, sorted. Each is provided by the engine, never by a module.
pub const CAPABILITIES: &[&str] = &[
    "animation",  // clip playback and blending (`AnimationPlayback`)
    "audio",      // sound and music playback (`SoundPlayback`)
    "camera",     // `GameCamera`
    "input",      // input actions (`register_action`)
    "networking", // server-authoritative netcode (`net` contract)
    "physics",    // `PhysicsBody` simulation
    "storage",    // `SaveStore` documents
    "ui",         // UI toolkit panels (`register_panel`)
];

/// True when `name` is a known capability.
pub fn is_capability(name: &str) -> bool {
    CAPABILITIES.contains(&name)
}
