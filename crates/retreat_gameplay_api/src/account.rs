//! Accounts and characters (crawler plan, phase 1): who plays, and which saved character a body is.
//!
//! An account (a dev login by name until platform accounts exist) has several characters, each
//! bound to a world (one server). The `account` module owns accounts and the list of characters;
//! when a player starts playing one it writes [`CharacterEntered`], and the module that spawns
//! bodies gives the body that character's [`PlayingCharacter`] and its saved place. Records of a
//! character live under [`character_record_owner`], one part per module.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

use crate::PlayerId;

/// An account: its login name in lowercase (`alice`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

/// A character, unique on the server: `<account>-<number>` (`alice-1`). Lowercase letters, digits
/// and `-`, so it is also a record owner segment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CharacterId(pub String);

/// The saved character a player's body is (on the body; shared state).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayingCharacter {
    pub id: CharacterId,
    /// The name players see (`Bob`).
    pub name: String,
    pub account: AccountId,
}

impl NetState for PlayingCharacter {
    const ID: &'static str = "gameplay.playing_character";
}

/// A player starts playing a character (written by the account module where the game is
/// decided): spawn its body at its saved place.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct CharacterEntered {
    pub player: PlayerId,
    pub character: PlayingCharacter,
}

/// A player stops playing a character but stays connected (logs out to the character list): save
/// the character's place and remove its body. A disconnect is the engine's `NetClientLeft`.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct CharacterLeft {
    pub player: PlayerId,
    pub character: CharacterId,
}

/// The record owner of an account: `account/<id>`.
pub fn account_record_owner(account: &AccountId) -> String {
    format!("account/{}", account.0)
}

/// The record owner of a character: `character/<id>`. Each module keeps its part of the character
/// there; deleting the character deletes every part.
pub fn character_record_owner(character: &CharacterId) -> String {
    format!("character/{}", character.0)
}
