//! What every weapon module does the same way. A library, not a module: it registers nothing and
//! has no state of its own. A weapon module keeps its cooldown, its numbers and what a shot does;
//! this crate gives it:
//! - [`rules`]: when a shot or a reload is allowed, and one tick of a weapon's timers (pure);
//! - [`player`]: who a command acts for and who may fight;
//! - [`inventory`]: a player's entry in the contract's `Weapons`;
//! - [`trigger`]: this machine's buttons and aim, turned into what to send;
//! - [`commands`]: the shared parts of `<weapon>.reload` and `assert.<weapon>_*`.

pub mod commands;
pub mod inventory;
pub mod player;
pub mod rules;
pub mod trigger;

pub use commands::{assert_weapon, reload_weapon};
pub use inventory::{count_hits, equip, loaded};
pub use player::{actor, character_of, dead_players, may_fight, refused};
pub use rules::{
    LATE_INPUT_TOLERANCE_TICKS, WeaponTicks, advance_weapon, ready_to_send, try_fire, try_reload,
};
pub use trigger::{Trigger, TriggerOrder, aim_angles, issue, pull_trigger};
