//! The account module's numbers.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use retreat_engine_api::is_valid_record_segment;

/// Most characters an account may have in one world that a session can carry (its shared state
/// must fit one network value).
pub const MOST_CHARACTERS_PER_WORLD: usize = 8;

/// Insert before adding the plugin to change the defaults (the same on server and clients).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct AccountConfig {
    /// The world (server) characters made here belong to: lowercase letters, digits, `_`, `-`.
    pub world: String,
    /// Most characters of one account in this world (placeholder number).
    pub max_characters: usize,
    /// How many letters and digits a login or character name has (placeholder numbers).
    pub name_length: RangeInclusive<usize>,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            world: "dev".to_string(),
            max_characters: 5,
            name_length: 3..=16,
        }
    }
}

impl AccountConfig {
    /// What is wrong, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        if !is_valid_record_segment(&self.world) {
            return Err(format!(
                "world `{}` must be lowercase letters, digits, `_` and `-`",
                self.world
            ));
        }
        if !(1..=MOST_CHARACTERS_PER_WORLD).contains(&self.max_characters) {
            return Err(format!(
                "max_characters must be 1 to {MOST_CHARACTERS_PER_WORLD}, got {}",
                self.max_characters
            ));
        }
        let (shortest, longest) = (*self.name_length.start(), *self.name_length.end());
        if shortest == 0 || longest < shortest || longest > 32 {
            return Err(format!(
                "name_length must be within 1 to 32 characters, got {shortest}..={longest}"
            ));
        }
        Ok(())
    }
}
