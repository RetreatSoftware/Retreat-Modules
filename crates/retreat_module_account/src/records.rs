//! The account module's records (its part `account` of each owner) and the atomic changes to them.
//!
//! - `account/<account>`: [`AccountRecord`], every character of the account in every world and
//!   the number the next one gets.
//! - `character/<id>`: [`CharacterRecord`], name, account and world of one character.
//! - `world/<world>`: [`WorldNames`], the character names taken in that world (lowercase), so a
//!   name is unique per world.

use std::collections::BTreeMap;

use retreat_engine_api::serde_json::{self, Value};
use retreat_engine_api::{RecordBatch, RecordKey, RecordStore};
use retreat_gameplay_api::{AccountId, CharacterId, account_record_owner, character_record_owner};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::config::AccountConfig;
use crate::names::{name_key, name_problem};
use crate::session::CharacterSummary;

/// This module's part of every owner it writes.
pub(crate) const RECORD_PART: &str = "account";
/// The data version of every record of this module.
const RECORD_VERSION: u32 = 1;

/// One character as its account lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AccountCharacter {
    pub id: String,
    pub name: String,
    pub world: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AccountRecord {
    pub characters: Vec<AccountCharacter>,
    /// The number of the next character (`<account>-<number>`); never reused.
    pub next_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CharacterRecord {
    pub name: String,
    pub account: String,
    pub world: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorldNames {
    /// Lowercase name -> character id.
    pub names: BTreeMap<String, String>,
}

fn world_owner(world: &str) -> String {
    format!("world/{world}")
}

fn read<T: DeserializeOwned + Default>(store: &RecordStore, owner: &str) -> Result<T, String> {
    let key = RecordKey::new(owner, RECORD_PART);
    let Some(record) = store.read(&key).map_err(|error| error.to_string())? else {
        return Ok(T::default());
    };
    record
        .check_not_newer_than(&key, RECORD_VERSION)
        .map_err(|error| error.to_string())?;
    serde_json::from_value(record.data).map_err(|error| {
        format!(
            "record `{key}` does not hold what the account module wrote ({error}); fix or delete it"
        )
    })
}

fn value(data: &impl Serialize) -> Value {
    serde_json::to_value(data).expect("account records are plain data")
}

/// The account's characters in `world`, in the order they were made.
pub(crate) fn characters_in_world(
    store: &RecordStore,
    account: &AccountId,
    world: &str,
) -> Result<Vec<CharacterSummary>, String> {
    let record: AccountRecord = read(store, &account_record_owner(account))?;
    Ok(record
        .characters
        .into_iter()
        .filter(|character| character.world == world)
        .map(|character| CharacterSummary {
            id: CharacterId(character.id),
            name: character.name,
        })
        .collect())
}

/// Makes a character called `name` for `account` in the configured world: one atomic batch over
/// the account, the character and the world's names. The error says why not (for the player).
pub(crate) fn create_character(
    store: &mut RecordStore,
    config: &AccountConfig,
    account: &AccountId,
    name: &str,
) -> Result<CharacterSummary, String> {
    if let Some(problem) = name_problem(name, &config.name_length) {
        return Err(problem);
    }
    let account_owner = account_record_owner(account);
    let mut record: AccountRecord = read(store, &account_owner)?;
    let in_world = record
        .characters
        .iter()
        .filter(|character| character.world == config.world)
        .count();
    if in_world >= config.max_characters {
        return Err(format!(
            "an account has at most {} characters in this world; delete one first",
            config.max_characters
        ));
    }
    let names_owner = world_owner(&config.world);
    let mut names: WorldNames = read(store, &names_owner)?;
    if names.names.contains_key(&name_key(name)) {
        return Err(format!("the name {name} is taken in this world"));
    }
    record.next_number += 1;
    let id = CharacterId(format!("{}-{}", account.0, record.next_number));
    record.characters.push(AccountCharacter {
        id: id.0.clone(),
        name: name.to_string(),
        world: config.world.clone(),
    });
    names.names.insert(name_key(name), id.0.clone());
    let character = CharacterRecord {
        name: name.to_string(),
        account: account.0.clone(),
        world: config.world.clone(),
    };
    store
        .write(
            RecordBatch::new()
                .put(
                    RecordKey::new(account_owner, RECORD_PART),
                    RECORD_VERSION,
                    value(&record),
                )
                .put(
                    RecordKey::new(character_record_owner(&id), RECORD_PART),
                    RECORD_VERSION,
                    value(&character),
                )
                .put(
                    RecordKey::new(names_owner, RECORD_PART),
                    RECORD_VERSION,
                    value(&names),
                ),
        )
        .map_err(|error| error.to_string())?;
    Ok(CharacterSummary {
        id,
        name: name.to_string(),
    })
}

/// Deletes a character of `account` in the configured world: removes it from the account and the
/// world's names and deletes every module's record of it, in one atomic batch.
pub(crate) fn delete_character(
    store: &mut RecordStore,
    config: &AccountConfig,
    account: &AccountId,
    character: &CharacterId,
) -> Result<(), String> {
    let account_owner = account_record_owner(account);
    let mut record: AccountRecord = read(store, &account_owner)?;
    let Some(position) = record
        .characters
        .iter()
        .position(|listed| listed.id == character.0 && listed.world == config.world)
    else {
        return Err(format!(
            "character {} is not one of this account's characters in this world",
            character.0
        ));
    };
    let removed = record.characters.remove(position);
    let names_owner = world_owner(&config.world);
    let mut names: WorldNames = read(store, &names_owner)?;
    names.names.remove(&name_key(&removed.name));
    store
        .write(
            RecordBatch::new()
                .put(
                    RecordKey::new(account_owner, RECORD_PART),
                    RECORD_VERSION,
                    value(&record),
                )
                .put(
                    RecordKey::new(names_owner, RECORD_PART),
                    RECORD_VERSION,
                    value(&names),
                )
                .delete_owner(character_record_owner(character)),
        )
        .map_err(|error| error.to_string())
}
