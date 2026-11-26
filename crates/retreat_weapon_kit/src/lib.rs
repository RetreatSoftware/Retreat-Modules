//! What every weapon module does the same way. A library, not a module: it registers nothing and
//! has no state of its own. A weapon module keeps its cooldown, its numbers and what a shot does;
//! this crate gives it:
//! - [`rules`]: when a shot or a reload is allowed, and one tick of a weapon's timers (pure);
//! - [`player`]: who a command acts for and who may fight;
//! - [`inventory`]: a player's entry in the contract's `Weapons`;
//! - [`trigger`]: this machine's buttons and aim, turned into what to send;
//! - [`sound`]: the reload sound;
//! - [`commands`]: the shared parts of `<weapon>.reload` and `assert.<weapon>_*`.

pub mod commands;
pub mod inventory;
pub mod player;
pub mod rules;
pub mod sound;
pub mod trigger;

pub use commands::{assert_weapon, reload_weapon};
pub use inventory::{count_hits, equip, loaded};
pub use player::{dead_players, may_fight};
pub use retreat_gameplay_api::{actor, character_of, issue, refused};
pub use rules::{
    LATE_INPUT_TOLERANCE_TICKS, WeaponTicks, advance_weapon, ready_to_send, try_fire, try_reload,
};
pub use sound::play_reload;
pub use trigger::{Trigger, TriggerMemory, TriggerOrder, aim_angles, pull_trigger};
