//! Sessions: who is logged in as which account, as shared state on `account/<player>`.

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use retreat_engine_api::{
    NetClientJoined, NetClientLeft, NetClientRejoined, NetPlayerIds, NetState, RecordStore,
    RemoveNetEntity, RetiredEntity,
};
use retreat_gameplay_api::{AccountId, CharacterId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::config::AccountConfig;
use crate::identity::account_of_player_id;
use crate::records::characters_in_world;

/// One character as a session lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: CharacterId,
    pub name: String,
}

/// A logged-in player (shared state on the entity [`session_entity_name`]).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSession {
    pub player: PlayerId,
    pub account: AccountId,
    /// The world (server) the characters belong to.
    pub world: String,
    /// The account's characters in this world, in the order they were made.
    pub characters: Vec<CharacterSummary>,
    /// The character being played, if any.
    pub playing: Option<CharacterId>,
    /// The last thing the player should read (why a request was refused); empty: nothing.
    pub notice: String,
}

impl NetState for AccountSession {
    const ID: &'static str = "account.session";
}

impl AccountSession {
    /// The listed character with this id.
    pub fn character(&self, id: &CharacterId) -> Option<&CharacterSummary> {
        self.characters.iter().find(|character| character.id == *id)
    }

    /// The listed character with this name (ignoring case).
    pub fn character_named(&self, name: &str) -> Option<&CharacterSummary> {
        self.characters
            .iter()
            .find(|character| character.name.eq_ignore_ascii_case(name))
    }
}

/// The entity name of a player's session: `account/<player id>`.
pub fn session_entity_name(player: PlayerId) -> String {
    format!("account/{}", player.0)
}

/// The live session entity of `player`.
pub(crate) fn session_of(world: &mut World, player: PlayerId) -> Option<Entity> {
    let mut sessions = world.query_filtered::<(Entity, &AccountSession), (
        Without<RetiredEntity>,
        Without<RemoveNetEntity>,
    )>();
    sessions
        .iter(world)
        .find(|(_, session)| session.player == player)
        .map(|(entity, _)| entity)
}

/// Starts the session of `player` as `account`, listing its characters in this world. Needs the
/// server's records.
pub(crate) fn open_session(
    world: &mut World,
    player: PlayerId,
    account: AccountId,
) -> Result<(), String> {
    if session_of(world, player).is_some() {
        return Err(format!("player {} is already logged in", player.0));
    }
    let config = world.resource::<AccountConfig>().clone();
    let Some(store) = world.get_resource::<RecordStore>() else {
        return Err("this app has no server records, so it cannot log anyone in".to_string());
    };
    let characters = characters_in_world(store, &account, &config.world)?;
    world.spawn((
        Name::new(session_entity_name(player)),
        AccountSession {
            player,
            account,
            world: config.world,
            characters,
            playing: None,
            notice: String::new(),
        },
    ));
    Ok(())
}

/// A takeover or a reconnect (`NetClientRejoined`): the player's session, still playing what it
/// played, moves to the new client id.
pub(crate) fn move_sessions_of_rejoined_players(
    world: &mut World,
    mut cursor: Local<MessageCursor<NetClientRejoined>>,
) {
    let rejoined: Vec<NetClientRejoined> = cursor
        .read(world.resource::<Messages<NetClientRejoined>>())
        .copied()
        .collect();
    for message in rejoined {
        let Some(old) = session_of(world, PlayerId(message.previous_client_id)) else {
            continue;
        };
        let mut session = world
            .get::<AccountSession>(old)
            .expect("found with this component")
            .clone();
        world.entity_mut(old).insert(RemoveNetEntity);
        session.player = PlayerId(message.client_id);
        world.spawn((Name::new(session_entity_name(session.player)), session));
    }
}

/// A client that joined (its login name was let in) gets its session.
pub(crate) fn open_sessions_of_joined_clients(
    world: &mut World,
    mut cursor: Local<MessageCursor<NetClientJoined>>,
) {
    let joined: Vec<u32> = cursor
        .read(world.resource::<Messages<NetClientJoined>>())
        .map(|message| message.client_id)
        .collect();
    for client_id in joined {
        let player = PlayerId(client_id);
        // A session moved here by a takeover in this tick is already open.
        if session_of(world, player).is_some() {
            continue;
        }
        let account = world
            .resource::<NetPlayerIds>()
            .player_of(client_id)
            .and_then(account_of_player_id);
        let Some(account) = account else {
            warn!("account: client {client_id} joined without a dev login; it gets no session");
            continue;
        };
        if let Err(problem) = open_session(world, player, account) {
            warn!("account: client {client_id} gets no session: {problem}");
        }
    }
}

/// A client that left ends its session (the spawn module saves where its character was).
pub(crate) fn close_sessions_of_left_clients(
    world: &mut World,
    mut cursor: Local<MessageCursor<NetClientLeft>>,
) {
    let left: Vec<u32> = cursor
        .read(world.resource::<Messages<NetClientLeft>>())
        .map(|message| message.client_id)
        .collect();
    for client_id in left {
        if let Some(session) = session_of(world, PlayerId(client_id)) {
            world.entity_mut(session).insert(RemoveNetEntity);
        }
    }
}
