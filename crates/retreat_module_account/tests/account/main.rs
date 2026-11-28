//! The account module with only Bevy and the contract crates: names, the dev login, records,
//! routing between client and server, sessions across joins and takeovers, and the screens. With
//! the real engine (network, UI) it is checked in the crawler testbed (Dungeon-Crawler).

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, NetClientIdentity, NetClientJoined, NetClientLeft,
    NetClientRejoined, NetCommandSender, NetIdentityCheck, NetIdentityDecision, NetIdentityRequest,
    NetInputQueue, NetPlayerIds, RecordKey, RecordStore, RemoveNetEntity, run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_account::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .add_plugins(AccountModulePlugin);
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn failure(app: &mut App, line: &str) -> String {
    run(app, line)
        .err()
        .unwrap_or_else(|| panic!("`{line}` must fail"))
        .to_string()
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn session(app: &mut App, player: u32) -> AccountSession {
    let world = app.world_mut();
    let mut sessions = world.query_filtered::<&AccountSession, Without<RemoveNetEntity>>();
    sessions
        .iter(world)
        .find(|session| session.player == PlayerId(player))
        .cloned()
        .unwrap_or_else(|| panic!("player {player} has no session"))
}

fn messages<M: Message + Clone>(app: &App) -> Vec<M> {
    let messages = app.world().resource::<Messages<M>>();
    messages.get_cursor().read(messages).cloned().collect()
}

#[test]
fn names_are_letters_and_digits_of_a_length_starting_with_a_letter() {
    let length = 3..=8;
    assert_eq!(name_problem("Alice", &length), None);
    assert_eq!(name_problem("bob2", &length), None);
    assert!(name_problem("Al", &length).unwrap().contains("has 2"));
    assert!(
        name_problem("Alexandria", &length)
            .unwrap()
            .contains("has 10")
    );
    assert!(
        name_problem("Ann Lee", &length)
            .unwrap()
            .contains("other characters")
    );
    assert!(
        name_problem("Zoë", &length)
            .unwrap()
            .contains("other characters")
    );
    assert!(
        name_problem("2pac", &length)
            .unwrap()
            .contains("starts with a letter")
    );
    assert_eq!(account_id_for_login("Alice").0, "alice");
}

#[test]
fn the_dev_login_lets_a_valid_name_in_as_its_lowercase_account() {
    let check = DevNameLogin {
        name_length: 3..=16,
    };
    let decide = |token| {
        check.check(NetIdentityRequest {
            connection_id: 1,
            token,
        })
    };
    assert_eq!(
        decide(Some("Alice")),
        NetIdentityDecision::Allow {
            player_id: "dev:alice".to_string()
        }
    );
    assert!(matches!(decide(None), NetIdentityDecision::Deny { .. }));
    assert!(matches!(
        decide(Some("a b")),
        NetIdentityDecision::Deny { .. }
    ));
    assert_eq!(
        account_of_player_id("dev:alice"),
        Some(AccountId("alice".to_string()))
    );
    assert_eq!(account_of_player_id("guest-3"), None);
    assert_eq!(
        player_id_for_account(&AccountId("bob".to_string())),
        "dev:bob"
    );
}

#[test]
fn the_screen_follows_the_session() {
    assert_eq!(account_screen(None, false), AccountScreen::Login);
    assert_eq!(account_screen(None, true), AccountScreen::Joining);
    let mut session = AccountSession {
        player: PlayerId(0),
        account: AccountId("alice".to_string()),
        world: "dev".to_string(),
        characters: Vec::new(),
        playing: None,
        notice: String::new(),
    };
    assert_eq!(
        account_screen(Some(&session), true),
        AccountScreen::Characters
    );
    session.playing = Some(CharacterId("alice-1".to_string()));
    assert_eq!(account_screen(Some(&session), true), AccountScreen::Playing);
}

#[test]
fn solo_login_create_play_log_out_and_delete() {
    let mut app = app();
    ok(&mut app, "assert.account_screen login");
    ok(&mut app, "account.login Alice");
    assert_eq!(app.world().resource::<SoloPlayer>().0, Some(PlayerId(0)));
    ok(&mut app, "assert.account_screen characters");
    ok(&mut app, "account.create Bob");
    ok(&mut app, "account.create Ann");
    ok(&mut app, "assert.account_characters 0 Bob Ann");
    ok(&mut app, "assert.account_characters me Bob Ann");
    let listed = session(&mut app, 0).characters;
    assert_eq!(listed[0].id.0, "alice-1");
    assert_eq!(listed[1].id.0, "alice-2");
    // The picked character is the first; Next picks Ann.
    ok(&mut app, "account.pick_next");
    ok(&mut app, "account.play_picked");
    ok(&mut app, "assert.account_playing 0 Ann");
    ok(&mut app, "assert.account_screen playing");
    let entered = messages::<CharacterEntered>(&app);
    assert_eq!(entered.len(), 1);
    assert_eq!(entered[0].player, PlayerId(0));
    assert_eq!(entered[0].character.name, "Ann");
    assert_eq!(entered[0].character.account.0, "alice");
    let error = failure(&mut app, "account.play alice-1");
    assert!(error.contains("already playing alice-2"), "{error}");
    let error = failure(&mut app, "account.delete alice-2");
    assert!(error.contains("while it is played"), "{error}");
    ok(&mut app, "account.logout");
    ok(&mut app, "assert.account_playing 0 nobody");
    let left = messages::<CharacterLeft>(&app);
    assert_eq!(left[0].character.0, "alice-2");
    // Delete asks first: one press arms, the second deletes the picked (Ann, still picked).
    ok(&mut app, "account.delete_picked");
    ok(&mut app, "assert.account_characters 0 Bob Ann");
    ok(&mut app, "account.delete_picked");
    ok(&mut app, "assert.account_characters 0 Bob");
}

#[test]
fn records_keep_the_account_and_names_are_unique_per_world_ignoring_case() {
    let mut app = app();
    ok(&mut app, "account.login alice");
    ok(&mut app, "account.create Bob");
    let error = failure(&mut app, "account.create BOB");
    assert!(error.contains("the name BOB is taken"), "{error}");
    assert!(session(&mut app, 0).notice.contains("taken"));
    let error = failure(&mut app, "account.create B");
    assert!(error.contains("has 1"), "{error}");
    let store = app.world().resource::<RecordStore>();
    let account = store
        .read(&RecordKey::new("account/alice", "account"))
        .unwrap()
        .unwrap();
    assert_eq!(account.data["next_number"], 1);
    assert_eq!(account.data["characters"][0]["name"], "Bob");
    assert_eq!(account.data["characters"][0]["world"], "dev");
    let names = store
        .read(&RecordKey::new("world/dev", "account"))
        .unwrap()
        .unwrap();
    assert_eq!(names.data["names"]["bob"], "alice-1");
    assert!(
        store
            .read(&RecordKey::new("character/alice-1", "account"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn deleting_a_character_removes_every_module_part_and_frees_its_name() {
    let mut app = app();
    ok(&mut app, "account.login alice");
    ok(&mut app, "account.create Bob");
    // Another module's record of the character (the spawn module's saved place).
    app.world_mut()
        .resource_mut::<RecordStore>()
        .put(
            RecordKey::new("character/alice-1", "player_spawn"),
            1,
            retreat_engine_api::serde_json::json!({"feet": [1, 0, 1], "yaw": 0}),
        )
        .unwrap();
    ok(&mut app, "account.delete alice-1");
    let store = app.world().resource::<RecordStore>();
    assert!(store.parts("character/alice-1").unwrap().is_empty());
    ok(&mut app, "account.create Bob");
    ok(&mut app, "assert.account_characters 0 Bob");
    assert_eq!(
        session(&mut app, 0).characters[0].id.0,
        "alice-2",
        "numbers are never reused"
    );
}

#[test]
fn an_account_has_at_most_max_characters_in_a_world() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .insert_resource(AccountConfig {
            max_characters: 2,
            ..AccountConfig::default()
        })
        .add_plugins(AccountModulePlugin);
    ok(&mut app, "account.login alice");
    ok(&mut app, "account.create Bob");
    ok(&mut app, "account.create Ann");
    let error = failure(&mut app, "account.create Cid");
    assert!(error.contains("at most 2 characters"), "{error}");
}

/// A server app where client 3 joined as `dev:alice`.
fn server_with_alice() -> App {
    let mut app = app();
    app.world_mut()
        .resource_mut::<NetPlayerIds>()
        .0
        .insert(3, "dev:alice".to_string());
    app.world_mut()
        .write_message(NetClientJoined { client_id: 3 });
    tick(&mut app);
    app
}

fn as_client(app: &mut App, client: u32, line: &str) -> CommandResult {
    app.insert_resource(NetCommandSender(Some(client)));
    let result = run(app, line);
    app.insert_resource(NetCommandSender(None));
    result
}

#[test]
fn the_server_opens_a_session_for_a_joined_login_and_decides_for_its_sender() {
    let mut app = server_with_alice();
    assert_eq!(session(&mut app, 3).account.0, "alice");
    as_client(&mut app, 3, "account.create Bob").unwrap();
    // A refusal is no error for a network input: the player reads it as the notice.
    as_client(&mut app, 3, "account.create Bob").unwrap();
    assert!(session(&mut app, 3).notice.contains("taken"));
    as_client(&mut app, 3, "account.play alice-1").unwrap();
    assert_eq!(
        session(&mut app, 3).playing,
        Some(CharacterId("alice-1".to_string()))
    );
    // The client leaves: its session ends.
    app.world_mut()
        .write_message(NetClientLeft { client_id: 3 });
    tick(&mut app);
    let world = app.world_mut();
    let live = world
        .query_filtered::<&AccountSession, Without<RemoveNetEntity>>()
        .iter(world)
        .count();
    assert_eq!(live, 0);
}

#[test]
fn a_takeover_moves_the_session_still_playing_to_the_new_connection() {
    let mut app = server_with_alice();
    as_client(&mut app, 3, "account.create Bob").unwrap();
    as_client(&mut app, 3, "account.play alice-1").unwrap();
    // The engine closed connection 3 because client 5 logged in as alice.
    app.world_mut().resource_mut::<NetPlayerIds>().0.remove(&3);
    app.world_mut()
        .resource_mut::<NetPlayerIds>()
        .0
        .insert(5, "dev:alice".to_string());
    app.world_mut()
        .write_message(NetClientLeft { client_id: 3 });
    app.world_mut()
        .write_message(NetClientJoined { client_id: 5 });
    app.world_mut().write_message(NetClientRejoined {
        client_id: 5,
        previous_client_id: 3,
    });
    tick(&mut app);
    let moved = session(&mut app, 5);
    assert_eq!(moved.playing, Some(CharacterId("alice-1".to_string())));
    let world = app.world_mut();
    let live: Vec<PlayerId> = world
        .query_filtered::<&AccountSession, Without<RemoveNetEntity>>()
        .iter(world)
        .map(|session| session.player)
        .collect();
    assert_eq!(live, [PlayerId(5)]);
}

#[test]
fn on_a_client_requests_go_to_the_server_and_predictions_do_nothing() {
    let mut app = app();
    app.world_mut().resource_mut::<NetInputQueue>().accepting = true;
    // Logging in on a client sets the hello's token.
    ok(&mut app, "account.login Alice");
    assert_eq!(
        app.world().resource::<NetClientIdentity>().token.as_deref(),
        Some("Alice")
    );
    ok(&mut app, "assert.account_screen joining");
    let error = failure(&mut app, "account.login Other");
    assert!(error.contains("already sent its login"), "{error}");
    ok(&mut app, "account.create Bob");
    assert_eq!(
        app.world().resource::<NetInputQueue>().lines,
        ["account.create Bob"]
    );
    // The client's own prediction of that line (sent by itself) changes nothing.
    as_client(&mut app, 7, "account.create Bob").unwrap();
    assert_eq!(app.world().resource::<NetInputQueue>().lines.len(), 1);
    assert!(
        app.world()
            .resource::<RecordStore>()
            .parts("character/alice-1")
            .unwrap()
            .is_empty()
    );
}
