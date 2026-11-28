//! `account.*` commands that change an account, the one routing rule they follow, and the
//! assertions.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, NetClientIdentity, NetCommandSender,
    NetInputQueue, NetLocalClient, RecordStore,
};
use retreat_gameplay_api::{
    CharacterEntered, CharacterId, CharacterLeft, PlayerId, PlayingCharacter, SoloPlayer, actor,
    local_player_id,
};

use crate::config::AccountConfig;
use crate::names::{account_id_for_login, name_problem};
use crate::records::{create_character, delete_character};
use crate::screen::AccountPicker;
use crate::session::{AccountSession, open_session, session_of};

/// The player id of the solo player (a game without a network).
pub(crate) const SOLO_PLAYER: PlayerId = PlayerId(0);

/// Where a command that changes an account goes (the one rule, see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// A network client's own request (a button, a field, a script line): send it to the server.
    SendToServer,
    /// A network client running its own request as a prediction: the server decides, so nothing.
    Ignore,
    /// The server, or a game without a network: decide.
    Decide,
}

pub(crate) fn route(world: &World) -> Route {
    let is_client = world.resource::<NetInputQueue>().accepting;
    let sent_by = world.resource::<NetCommandSender>().0;
    match (is_client, sent_by) {
        (true, None) => Route::SendToServer,
        (true, Some(_)) => Route::Ignore,
        (false, _) => Route::Decide,
    }
}

/// Sends `line` to the server as this player's input.
pub(crate) fn send_to_server(world: &mut World, line: String) {
    world.resource_mut::<NetInputQueue>().push(line);
}

/// Shows `why` to `player` (as its session's notice). A refusal is no error for a network input;
/// a script line fails with it.
fn refuse(world: &mut World, args: &CommandArgs, player: PlayerId, why: String) -> CommandResult {
    if let Some(session) = session_of(world, player) {
        world
            .get_mut::<AccountSession>(session)
            .expect("found with this component")
            .notice = why.clone();
    }
    if world.resource::<NetCommandSender>().0.is_some() {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(why))
    }
}

/// The session entity of the player a deciding command acts for.
fn deciding_session(
    world: &mut World,
    args: &CommandArgs,
) -> Result<(PlayerId, Entity), CommandError> {
    let player = actor(world, args)?;
    let session = session_of(world, player).ok_or_else(|| {
        args.failure(format!(
            "player {} is not logged in; log in with `account.login <name>`",
            player.0
        ))
    })?;
    Ok((player, session))
}

fn session(world: &World, entity: Entity) -> AccountSession {
    world
        .get::<AccountSession>(entity)
        .expect("a session entity has its session")
        .clone()
}

fn clear_notice(world: &mut World, entity: Entity) {
    let mut session = world
        .get_mut::<AccountSession>(entity)
        .expect("a session entity has its session");
    if !session.notice.is_empty() {
        session.notice.clear();
    }
}

/// `account.login <name>`: on a network client the name becomes the login of its hello (the
/// client joins with it); without a network it logs in the solo player.
pub(crate) fn login_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let name: String = args.parse(0, "name")?;
    let length = world.resource::<AccountConfig>().name_length.clone();
    if let Some(problem) = name_problem(&name, &length) {
        world.resource_mut::<AccountPicker>().local_notice = problem.clone();
        return Err(args.failure(problem));
    }
    if world.resource::<NetInputQueue>().accepting {
        let mut identity = world.resource_mut::<NetClientIdentity>();
        if identity.token.is_some() {
            return Err(args.failure(
                "this client already sent its login; reload the game to log in as someone else",
            ));
        }
        identity.token = Some(name.clone());
        let mut picker = world.resource_mut::<AccountPicker>();
        picker.login_sent = Some(name);
        picker.local_notice.clear();
        return Ok(CommandOutcome::Continue);
    }
    open_session(world, SOLO_PLAYER, account_id_for_login(&name))
        .map_err(|problem| args.failure(problem))?;
    world.insert_resource(SoloPlayer(Some(SOLO_PLAYER)));
    world.resource_mut::<AccountPicker>().local_notice.clear();
    Ok(CommandOutcome::Continue)
}

/// `account.create <character name>`.
pub(crate) fn create_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let name: String = args.parse(0, "character name")?;
    match route(world) {
        Route::SendToServer => {
            send_to_server(world, format!("account.create {name}"));
            return Ok(CommandOutcome::Continue);
        }
        Route::Ignore => return Ok(CommandOutcome::Continue),
        Route::Decide => {}
    }
    let (player, entity) = deciding_session(world, args)?;
    let account = session(world, entity).account;
    let config = world.resource::<AccountConfig>().clone();
    let Some(mut store) = world.get_resource_mut::<RecordStore>() else {
        return Err(args.failure("this app has no server records"));
    };
    match create_character(&mut store, &config, &account, &name) {
        Ok(character) => {
            let mut session = world
                .get_mut::<AccountSession>(entity)
                .expect("a session entity has its session");
            session.characters.push(character);
            session.notice.clear();
            Ok(CommandOutcome::Continue)
        }
        Err(why) => refuse(world, args, player, why),
    }
}

/// Starts playing `character` of `player`'s session: the shared rule of `account.play` and
/// `account.play_picked`. The error is for the player.
pub(crate) fn play_character(
    world: &mut World,
    player: PlayerId,
    entity: Entity,
    character: &CharacterId,
) -> Result<(), String> {
    let current = session(world, entity);
    if let Some(playing) = &current.playing {
        return Err(format!("already playing {}; log out first", playing.0));
    }
    let Some(listed) = current.character(character).cloned() else {
        return Err(format!(
            "character {} is not one of this account's characters in this world",
            character.0
        ));
    };
    {
        let mut session = world
            .get_mut::<AccountSession>(entity)
            .expect("a session entity has its session");
        session.playing = Some(listed.id.clone());
        session.notice.clear();
    }
    world.write_message(CharacterEntered {
        player,
        character: PlayingCharacter {
            id: listed.id,
            name: listed.name,
            account: current.account,
        },
    });
    Ok(())
}

/// Deletes `character` of `player`'s session: the shared rule of `account.delete` and
/// `account.delete_picked`. The error is for the player.
pub(crate) fn delete_listed_character(
    world: &mut World,
    entity: Entity,
    character: &CharacterId,
) -> Result<(), String> {
    let current = session(world, entity);
    if current.playing.as_ref() == Some(character) {
        return Err("a character cannot be deleted while it is played; log out first".to_string());
    }
    let config = world.resource::<AccountConfig>().clone();
    let Some(mut store) = world.get_resource_mut::<RecordStore>() else {
        return Err("this app has no server records".to_string());
    };
    delete_character(&mut store, &config, &current.account, character)?;
    let mut session = world
        .get_mut::<AccountSession>(entity)
        .expect("a session entity has its session");
    session.characters.retain(|listed| listed.id != *character);
    session.notice.clear();
    Ok(())
}

/// `account.play <character id>`.
pub(crate) fn play_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let character = CharacterId(args.parse(0, "character id")?);
    match route(world) {
        Route::SendToServer => {
            send_to_server(world, format!("account.play {}", character.0));
            return Ok(CommandOutcome::Continue);
        }
        Route::Ignore => return Ok(CommandOutcome::Continue),
        Route::Decide => {}
    }
    let (player, entity) = deciding_session(world, args)?;
    match play_character(world, player, entity, &character) {
        Ok(()) => Ok(CommandOutcome::Continue),
        Err(why) => refuse(world, args, player, why),
    }
}

/// `account.logout`: stop playing (the character's place is saved) and return to the list.
pub(crate) fn logout_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    match route(world) {
        Route::SendToServer => {
            send_to_server(world, "account.logout".to_string());
            return Ok(CommandOutcome::Continue);
        }
        Route::Ignore => return Ok(CommandOutcome::Continue),
        Route::Decide => {}
    }
    let (player, entity) = deciding_session(world, args)?;
    let Some(character) = session(world, entity).playing else {
        return refuse(
            world,
            args,
            player,
            "no character is being played".to_string(),
        );
    };
    world
        .get_mut::<AccountSession>(entity)
        .expect("a session entity has its session")
        .playing = None;
    clear_notice(world, entity);
    world.write_message(CharacterLeft { player, character });
    Ok(CommandOutcome::Continue)
}

/// `account.delete <character id>`.
pub(crate) fn delete_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let character = CharacterId(args.parse(0, "character id")?);
    match route(world) {
        Route::SendToServer => {
            send_to_server(world, format!("account.delete {}", character.0));
            return Ok(CommandOutcome::Continue);
        }
        Route::Ignore => return Ok(CommandOutcome::Continue),
        Route::Decide => {}
    }
    let (player, entity) = deciding_session(world, args)?;
    match delete_listed_character(world, entity, &character) {
        Ok(()) => Ok(CommandOutcome::Continue),
        Err(why) => refuse(world, args, player, why),
    }
}

/// The session of the player named by argument 0 (`me`: this machine's player), on any machine
/// (sessions are shared state).
fn session_for_assert(
    world: &mut World,
    args: &CommandArgs,
) -> Result<AccountSession, CommandError> {
    let named: String = args.parse(0, "player")?;
    let player = if named == "me" {
        local_player_id(
            *world.resource::<NetLocalClient>(),
            *world.resource::<SoloPlayer>(),
        )
        .ok_or_else(|| args.failure("this machine has no player yet (not joined or logged in)"))?
    } else {
        PlayerId(args.parse(0, "player")?)
    };
    let entity = session_of(world, player)
        .ok_or_else(|| args.failure(format!("player {} has no session", player.0)))?;
    Ok(session(world, entity))
}

/// `assert.account_characters <player> [character names...]`: the session lists exactly these
/// characters, in this order (no names: none).
pub(crate) fn assert_characters_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    if args.rest(0).is_empty() {
        return Err(
            args.failure("expected a player, then the character names (none: no characters)")
        );
    }
    let session = session_for_assert(world, args)?;
    let actual: Vec<&str> = session
        .characters
        .iter()
        .map(|character| character.name.as_str())
        .collect();
    let expected: Vec<&str> = args.rest(1).iter().map(String::as_str).collect();
    if actual == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "player {} lists characters {actual:?}, expected {expected:?}",
            session.player.0
        )))
    }
}

/// `assert.account_playing <player> <character name|nobody>`.
pub(crate) fn assert_playing_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let expected: String = args.parse(1, "character name")?;
    let session = session_for_assert(world, args)?;
    let actual = session
        .playing
        .as_ref()
        .and_then(|id| session.character(id))
        .map_or("nobody", |character| character.name.as_str());
    if actual.eq_ignore_ascii_case(&expected) {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "player {} is playing {actual}, expected {expected}",
            session.player.0
        )))
    }
}
