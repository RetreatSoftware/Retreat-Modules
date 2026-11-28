//! The account module's records (its part `account` of each owner) and the atomic changes to them.
//!
//! - `account/<account>`: [`AccountRecord`], every character of the account in every world and
//!   the number the next one gets.
//! - `character/<id>`: [`CharacterRecord`], name, account and world of one character.
//! - `world/<world>`: [`WorldNames`], the character names taken in that world (lowercase), so a
//!   name is unique per world.
//!
//! Every world server shares the records, and one account may be logged in on two worlds at once,
//! so the account record (and a world's names) may be changed by another server between reading
//! and writing: each change writes only if they are still as read (`expect_unchanged`), and is
//! decided again from fresh records when they were not, up to [`CONFLICT_ATTEMPTS`] times.

use std::collections::BTreeMap;

use retreat_engine_api::serde_json::{self, Value};
use retreat_engine_api::{RecordBatch, RecordError, RecordKey, RecordStore, StoredRecord};
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
/// How often a change is decided again when another world server changed its records meanwhile.
pub(crate) const CONFLICT_ATTEMPTS: usize = 4;

/// Why one attempt at a change did not write.
enum Attempt {
    /// Another server changed a record in between: decide again.
    Conflict,
    /// Refused or failed (the message is for the player).
    Failed(String),
}

impl From<String> for Attempt {
    fn from(message: String) -> Self {
        Attempt::Failed(message)
    }
}

fn written(result: Result<(), RecordError>) -> Result<(), Attempt> {
    match result {
        Ok(()) => Ok(()),
        Err(RecordError::Conflict { .. }) => Err(Attempt::Conflict),
        Err(error) => Err(Attempt::Failed(error.to_string())),
    }
}

/// Runs `attempt` until it is not refused by a conflict (see the module docs).
fn decide_until_written<T>(mut attempt: impl FnMut() -> Result<T, Attempt>) -> Result<T, String> {
    for _ in 0..CONFLICT_ATTEMPTS {
        match attempt() {
            Ok(done) => return Ok(done),
            Err(Attempt::Conflict) => continue,
            Err(Attempt::Failed(message)) => return Err(message),
        }
    }
    Err(format!(
        "the account changed on another world {CONFLICT_ATTEMPTS} times meanwhile; try again"
    ))
}

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
    read_as_stored(store, owner).map(|(data, _)| data)
}

/// The record of `owner` and the stored record it came from (for `expect_unchanged`).
fn read_as_stored<T: DeserializeOwned + Default>(
    store: &RecordStore,
    owner: &str,
) -> Result<(T, Option<StoredRecord>), String> {
    let key = RecordKey::new(owner, RECORD_PART);
    let Some(record) = store.read(&key).map_err(|error| error.to_string())? else {
        return Ok((T::default(), None));
    };
    record
        .check_not_newer_than(&key, RECORD_VERSION)
        .map_err(|error| error.to_string())?;
    let data = serde_json::from_value(record.data.clone()).map_err(|error| {
        format!(
            "record `{key}` does not hold what the account module wrote ({error}); fix or delete it"
        )
    })?;
    Ok((data, Some(record)))
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
    decide_until_written(|| try_create_character(store, config, account, name))
}

fn try_create_character(
    store: &mut RecordStore,
    config: &AccountConfig,
    account: &AccountId,
    name: &str,
) -> Result<CharacterSummary, Attempt> {
    let account_owner = account_record_owner(account);
    let (mut record, account_seen): (AccountRecord, _) = read_as_stored(store, &account_owner)?;
    let in_world = record
        .characters
        .iter()
        .filter(|character| character.world == config.world)
        .count();
    if in_world >= config.max_characters {
        return Err(Attempt::Failed(format!(
            "an account has at most {} characters in this world; delete one first",
            config.max_characters
        )));
    }
    let names_owner = world_owner(&config.world);
    let (mut names, names_seen): (WorldNames, _) = read_as_stored(store, &names_owner)?;
    if names.names.contains_key(&name_key(name)) {
        return Err(Attempt::Failed(format!(
            "the name {name} is taken in this world"
        )));
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
    written(
        store.write(
            RecordBatch::new()
                .expect_unchanged(
                    RecordKey::new(account_owner.clone(), RECORD_PART),
                    account_seen,
                )
                .expect_unchanged(RecordKey::new(names_owner.clone(), RECORD_PART), names_seen)
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
        ),
    )?;
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
    decide_until_written(|| try_delete_character(store, config, account, character))
}

fn try_delete_character(
    store: &mut RecordStore,
    config: &AccountConfig,
    account: &AccountId,
    character: &CharacterId,
) -> Result<(), Attempt> {
    let account_owner = account_record_owner(account);
    let (mut record, account_seen): (AccountRecord, _) = read_as_stored(store, &account_owner)?;
    let Some(position) = record
        .characters
        .iter()
        .position(|listed| listed.id == character.0 && listed.world == config.world)
    else {
        return Err(Attempt::Failed(format!(
            "character {} is not one of this account's characters in this world",
            character.0
        )));
    };
    let removed = record.characters.remove(position);
    let names_owner = world_owner(&config.world);
    let (mut names, names_seen): (WorldNames, _) = read_as_stored(store, &names_owner)?;
    names.names.remove(&name_key(&removed.name));
    written(
        store.write(
            RecordBatch::new()
                .expect_unchanged(
                    RecordKey::new(account_owner.clone(), RECORD_PART),
                    account_seen,
                )
                .expect_unchanged(RecordKey::new(names_owner.clone(), RECORD_PART), names_seen)
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
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_refused_by_a_conflict_is_decided_again_and_gives_up_with_a_reason() {
        let mut tries = 0;
        let done = decide_until_written(|| {
            tries += 1;
            if tries < 3 {
                Err(Attempt::Conflict)
            } else {
                Ok(tries)
            }
        });
        assert_eq!(done, Ok(3));
        let mut tries = 0;
        let error = decide_until_written::<()>(|| {
            tries += 1;
            Err(Attempt::Conflict)
        })
        .unwrap_err();
        assert_eq!(tries, CONFLICT_ATTEMPTS);
        assert!(error.contains("try again"), "{error}");
        let refused = decide_until_written::<()>(|| Err(Attempt::Failed("taken".to_string())));
        assert_eq!(refused, Err("taken".to_string()));
    }
}
