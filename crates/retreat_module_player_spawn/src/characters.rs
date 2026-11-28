//! Saved characters (crawler plan, phase 1; used when `PlayerSpawnConfig::spawn_on_join` is off):
//! a body appears when its player starts playing a character (`CharacterEntered`), at the place
//! the character was saved, and its place is saved when it stops (`CharacterLeft`), when its
//! player disconnects (`NetClientLeft`, the body leaves at once: decided with the user for the
//! safe city) and every `autosave_seconds`. A takeover (`NetClientRejoined` in the same tick as the
//! leave) hands the body to the new connection instead.
//!
//! The place is this module's record part `player_spawn` of `character/<id>`: `{feet: [x, y, z],
//! yaw}`. Saving needs the server's records, so it happens only where the game is decided.

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use retreat_engine_api::serde_json::{Value, json};
use retreat_engine_api::{
    NetClientLeft, NetClientRejoined, RecordKey, RecordStore, RemovePhysicsBody, RetiredEntity,
};
use retreat_gameplay_api::{
    CharacterEntered, CharacterLeft, CharacterSize, LookAngles, MapAreas, PlayerId,
    PlayingCharacter, SpawnPoint, SpawnPoints, character_record_owner, choose_spawn_point,
    ticks_from_seconds,
};

use crate::PlayerSpawnConfig;
use crate::spawn::{occupied_feet, spawn_character_named};

/// This module's part of a character's records.
pub const PLACE_RECORD_PART: &str = "player_spawn";
/// The data version of the saved place.
const PLACE_RECORD_VERSION: u32 = 1;

/// Where a character entering the world appears: its saved place when it is still a place of the
/// map (finite, and inside an area when the map has areas), otherwise the spawn point of the home
/// area (all spawn points for a map without areas) farthest from the others. Pure.
pub fn entry_place(
    saved: Option<SpawnPoint>,
    areas: &MapAreas,
    points: &[SpawnPoint],
    occupied: &[Vec3],
) -> Option<SpawnPoint> {
    let still_valid = |place: &SpawnPoint| {
        place.feet.is_finite()
            && place.yaw.is_finite()
            && (areas.0.is_empty() || areas.at(place.feet).is_some())
    };
    if let Some(place) = saved.filter(still_valid) {
        return Some(place);
    }
    let home_points = match areas.home() {
        Some(home) => MapAreas::spawn_points_in(points, home),
        None => points.to_vec(),
    };
    choose_spawn_point(&home_points, occupied).map(|index| home_points[index])
}

fn place_key(character: &PlayingCharacter) -> RecordKey {
    RecordKey::new(character_record_owner(&character.id), PLACE_RECORD_PART)
}

/// The saved place of a character, if there is one and it can be read.
fn saved_place(store: &RecordStore, character: &PlayingCharacter) -> Option<SpawnPoint> {
    let key = place_key(character);
    let record = match store.read(&key) {
        Ok(record) => record?,
        Err(error) => {
            warn!("player_spawn: {error}; the character starts at a spawn point");
            return None;
        }
    };
    if let Err(error) = record.check_not_newer_than(&key, PLACE_RECORD_VERSION) {
        warn!("player_spawn: {error}");
        return None;
    }
    let feet = record.data.get("feet")?.as_array()?;
    let number = |value: &Value| value.as_f64().map(|n| n as f32);
    let feet = Vec3::new(
        number(feet.first()?)?,
        number(feet.get(1)?)?,
        number(feet.get(2)?)?,
    );
    let yaw = number(record.data.get("yaw")?)?;
    Some(SpawnPoint { feet, yaw })
}

/// Saves where the body `entity` is, as its character's place (only where there are records).
fn save_place(world: &mut World, entity: Entity) {
    let size = *world.resource::<CharacterSize>();
    let Some(character) = world.get::<PlayingCharacter>(entity).cloned() else {
        return;
    };
    let Some(transform) = world.get::<Transform>(entity) else {
        return;
    };
    let feet = size.feet(transform.translation);
    let yaw = world.get::<LookAngles>(entity).map_or(0.0, |look| look.yaw);
    let Some(mut store) = world.get_resource_mut::<RecordStore>() else {
        return;
    };
    let data = json!({"feet": [feet.x, feet.y, feet.z], "yaw": yaw});
    if let Err(error) = store.put(place_key(&character), PLACE_RECORD_VERSION, data) {
        error!(
            "player_spawn: the place of {} was not saved: {error}",
            character.id.0
        );
    }
}

/// How many characters entered the world on this machine (the server): numbers body names.
#[derive(Resource, Debug, Default)]
pub(crate) struct CharacterEntries(u64);

/// The body name of a character's `entry`-th appearance: `character_<id>_<entry>`. A body that left
/// keeps its name for the engine's rollback window, so a character that comes back at once gets a
/// new name.
pub fn character_body_name(character: &PlayingCharacter, entry: u64) -> String {
    format!("character_{}_{entry}", character.id.0)
}

/// The live body of `player`.
fn body_of(world: &mut World, player: PlayerId) -> Option<Entity> {
    let mut bodies = world.query_filtered::<(Entity, &PlayerId), (
        Without<RetiredEntity>,
        Without<RemovePhysicsBody>,
    )>();
    bodies
        .iter(world)
        .find(|(_, found)| **found == player)
        .map(|(entity, _)| entity)
}

/// Spawns the body of a character that starts playing, at its saved place.
pub(crate) fn spawn_entered_characters(
    world: &mut World,
    mut cursor: Local<MessageCursor<CharacterEntered>>,
) {
    let entered: Vec<CharacterEntered> = cursor
        .read(world.resource::<Messages<CharacterEntered>>())
        .cloned()
        .collect();
    for message in entered {
        let saved = world
            .get_resource::<RecordStore>()
            .and_then(|store| saved_place(store, &message.character));
        let occupied = occupied_feet(world);
        let place = entry_place(
            saved,
            world.resource::<MapAreas>(),
            &world.resource::<SpawnPoints>().0,
            &occupied,
        );
        let Some(place) = place else {
            warn!(
                "player_spawn: {} cannot enter: no saved place and no spawn point; load a map first",
                message.character.name
            );
            continue;
        };
        let entry = {
            let mut entries = world.get_resource_or_insert_with(CharacterEntries::default);
            entries.0 += 1;
            entries.0
        };
        let name = character_body_name(&message.character, entry);
        match spawn_character_named(world, message.player, place, name) {
            Ok(body) => {
                world.entity_mut(body).insert(message.character);
            }
            Err(problem) => warn!(
                "player_spawn: {} cannot enter: {problem}",
                message.character.name
            ),
        }
    }
}

/// Saves and removes the body of a character that stopped playing.
pub(crate) fn remove_left_characters(
    world: &mut World,
    mut cursor: Local<MessageCursor<CharacterLeft>>,
) {
    let left: Vec<CharacterLeft> = cursor
        .read(world.resource::<Messages<CharacterLeft>>())
        .cloned()
        .collect();
    for message in left {
        let Some(body) = body_of(world, message.player) else {
            continue;
        };
        let is_that_character = world
            .get::<PlayingCharacter>(body)
            .is_some_and(|playing| playing.id == message.character);
        if is_that_character {
            save_place(world, body);
            world.entity_mut(body).insert(RemovePhysicsBody);
        }
    }
}

/// A player who disconnected: hands the body to the player's new connection when the same tick
/// has the rejoin (a takeover), otherwise saves its place and removes it at once.
pub(crate) fn handle_disconnected_characters(
    world: &mut World,
    mut left_cursor: Local<MessageCursor<NetClientLeft>>,
    mut rejoined_cursor: Local<MessageCursor<NetClientRejoined>>,
) {
    let left: Vec<u32> = left_cursor
        .read(world.resource::<Messages<NetClientLeft>>())
        .map(|message| message.client_id)
        .collect();
    let rejoined: Vec<NetClientRejoined> = rejoined_cursor
        .read(world.resource::<Messages<NetClientRejoined>>())
        .copied()
        .collect();
    for client_id in left {
        let Some(body) = body_of(world, PlayerId(client_id)) else {
            continue;
        };
        if world.get::<PlayingCharacter>(body).is_none() {
            continue;
        }
        let takeover = rejoined
            .iter()
            .find(|rejoin| rejoin.previous_client_id == client_id);
        match takeover {
            Some(rejoin) => {
                // The same character, played from the new connection, where it stands.
                world.entity_mut(body).insert(PlayerId(rejoin.client_id));
            }
            None => {
                save_place(world, body);
                world.entity_mut(body).insert(RemovePhysicsBody);
            }
        }
    }
}

/// Saves every played character's place every `autosave_seconds` (a crashed server loses at most
/// that much walking).
pub(crate) fn autosave_places(world: &mut World, mut ticks: Local<u64>) {
    if !world.contains_resource::<RecordStore>() {
        return;
    }
    let timestep = world.resource::<Time<Fixed>>().timestep();
    let seconds = world.resource::<PlayerSpawnConfig>().autosave_seconds;
    let every = ticks_from_seconds(seconds, timestep).max(1);
    *ticks += 1;
    if !(*ticks).is_multiple_of(u64::from(every)) {
        return;
    }
    let mut played = world.query_filtered::<(Entity, &PlayingCharacter), (
        Without<RetiredEntity>,
        Without<RemovePhysicsBody>,
    )>();
    let mut bodies: Vec<(String, Entity)> = played
        .iter(world)
        .map(|(entity, character)| (character.id.0.clone(), entity))
        .collect();
    bodies.sort();
    for (_, body) in bodies {
        save_place(world, body);
    }
}
