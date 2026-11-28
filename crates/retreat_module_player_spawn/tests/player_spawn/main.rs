//! Player spawning with only Bevy and the contract crates. With the real engine (bodies, network)
//! it is checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, NetClientJoined, NetClientLeft, NetClientRejoined, PhysicsBody,
    PhysicsBodyKind, RecordKey, RecordStore, RemovePhysicsBody, run_command_line, serde_json::json,
};
use retreat_gameplay_api::*;
use retreat_module_player_spawn::{PlayerSpawnConfig, PlayerSpawnModulePlugin, entry_place};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(PlayerSpawnModulePlugin);
    app.world_mut().resource_mut::<SpawnPoints>().0 = [20.0, -20.0]
        .map(|z| SpawnPoint {
            feet: Vec3::new(0.0, 0.0, z),
            yaw: z / 20.0,
        })
        .to_vec();
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

/// `(name, position, look yaw)` by player id.
fn characters(app: &mut App) -> Vec<(String, Vec3, f32)> {
    let world = app.world_mut();
    let mut query = world.query::<(&PlayerId, &Name, &Transform, &LookAngles, &PhysicsBody)>();
    let mut found: Vec<_> = query
        .iter(world)
        .map(|(id, name, transform, look, body)| {
            assert_eq!(body.kind, PhysicsBodyKind::Character);
            (*id, name.to_string(), transform.translation, look.yaw)
        })
        .collect();
    found.sort_by_key(|(id, ..)| *id);
    found
        .into_iter()
        .map(|(_, name, position, yaw)| (name, position, yaw))
        .collect()
}

#[test]
fn joined_clients_get_characters_on_different_points_facing_their_way() {
    let mut app = app();
    for client_id in [1, 0] {
        app.world_mut().write_message(NetClientJoined { client_id });
    }
    tick(&mut app);
    assert_eq!(
        characters(&mut app),
        [
            ("player_0".to_string(), Vec3::new(0.0, 0.9, 20.0), 1.0),
            ("player_1".to_string(), Vec3::new(0.0, 0.9, -20.0), -1.0),
        ]
    );
    tick(&mut app);
    ok(&mut app, "assert.spawn_count 2");
    assert_eq!(app.world().resource::<SoloPlayer>().0, None);
}

#[test]
fn a_client_that_leaves_loses_its_character() {
    let mut app = app();
    app.world_mut()
        .write_message(NetClientJoined { client_id: 0 });
    tick(&mut app);
    app.world_mut()
        .write_message(NetClientLeft { client_id: 0 });
    tick(&mut app);
    let world = app.world_mut();
    let removing = world
        .query_filtered::<(), With<RemovePhysicsBody>>()
        .iter(world)
        .count();
    assert_eq!(removing, 1);
    ok(&mut app, "assert.spawn_count 0");
}

#[test]
fn solo_spawns_and_marks_this_machines_player() {
    let mut app = app();
    ok(&mut app, "spawn.solo 0");
    assert_eq!(app.world().resource::<SoloPlayer>().0, Some(PlayerId(0)));
    let error = run(&mut app, "spawn.solo 1")
        .expect_err("twice")
        .to_string();
    assert!(error.contains("already the solo player"), "{error}");
    ok(&mut app, "spawn.leave 0");
    assert_eq!(app.world().resource::<SoloPlayer>().0, None);
}

#[test]
fn refusals_say_why() {
    let mut app = app();
    ok(&mut app, "spawn.join 1");
    for (line, expected) in [
        ("spawn.join 1", "already exists"),
        ("spawn.leave 5", "no player 5"),
        ("assert.spawn_count 3", "there are 1 players"),
    ] {
        let error = run(&mut app, line).expect_err(line).to_string();
        assert!(error.contains(expected), "{line}: {error}");
    }
    for id in 2..=MAX_PLAYERS {
        ok(&mut app, &format!("spawn.join {id}"));
    }
    let error = run(&mut app, "spawn.join 99")
        .expect_err("full")
        .to_string();
    assert!(error.contains("already 8 players"), "{error}");
    app.world_mut().resource_mut::<SpawnPoints>().0.clear();
    ok(&mut app, "spawn.leave 1");
}

#[test]
fn without_a_map_nobody_spawns() {
    let mut app = app();
    app.world_mut().resource_mut::<SpawnPoints>().0.clear();
    let error = run(&mut app, "spawn.join 1")
        .expect_err("no map")
        .to_string();
    assert!(error.contains("load a map first"), "{error}");
}

// Saved characters (`spawn_on_join: false`, the crawler).

fn city() -> MapAreas {
    MapAreas(vec![Area {
        id: AreaId("city".to_string()),
        name: "City".to_string(),
        rules: AreaRules {
            safe: true,
            pvp: false,
            tier: 0,
            max_entry_level: None,
        },
        min: Vec2::splat(-30.0),
        max: Vec2::splat(30.0),
        home: true,
    }])
}

fn crawler_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .insert_resource(PlayerSpawnConfig {
            spawn_on_join: false,
            ..PlayerSpawnConfig::default()
        })
        .add_plugins(PlayerSpawnModulePlugin);
    app.world_mut().resource_mut::<SpawnPoints>().0 = [20.0, -20.0]
        .map(|z| SpawnPoint {
            feet: Vec3::new(0.0, 0.0, z),
            yaw: z / 20.0,
        })
        .to_vec();
    app.insert_resource(city());
    app
}

fn bob() -> PlayingCharacter {
    PlayingCharacter {
        id: CharacterId("alice-1".to_string()),
        name: "Bob".to_string(),
        account: AccountId("alice".to_string()),
    }
}

fn enter(app: &mut App, player: u32) {
    app.world_mut().write_message(CharacterEntered {
        player: PlayerId(player),
        character: bob(),
    });
    tick(app);
}

fn body(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, (With<PlayingCharacter>, Without<RemovePhysicsBody>)>()
        .single(world)
        .expect("one played character")
}

fn saved(app: &App) -> Option<retreat_engine_api::serde_json::Value> {
    app.world()
        .resource::<RecordStore>()
        .read(&RecordKey::new("character/alice-1", "player_spawn"))
        .unwrap()
        .map(|record| record.data)
}

fn point(x: f32, z: f32) -> SpawnPoint {
    SpawnPoint {
        feet: Vec3::new(x, 0.0, z),
        yaw: 0.5,
    }
}

#[test]
fn a_character_enters_where_it_was_saved_if_that_is_still_a_place_of_the_map() {
    let points = [point(0.0, 20.0), point(0.0, -20.0), point(100.0, 0.0)];
    let areas = city();
    assert_eq!(
        entry_place(Some(point(3.0, 4.0)), &areas, &points, &[]),
        Some(point(3.0, 4.0))
    );
    // Saved outside every area (the map changed): the home area's spawn point farthest from others.
    let occupied = [Vec3::new(0.0, 0.0, 19.0)];
    assert_eq!(
        entry_place(Some(point(500.0, 0.0)), &areas, &points, &occupied),
        Some(point(0.0, -20.0))
    );
    assert_eq!(
        entry_place(None, &areas, &points, &[]),
        Some(point(0.0, 20.0)),
        "a new character starts in the home area, never at the point outside it"
    );
    let broken = SpawnPoint {
        feet: Vec3::NAN,
        yaw: 0.0,
    };
    assert_eq!(
        entry_place(Some(broken), &areas, &points, &[]),
        Some(point(0.0, 20.0))
    );
    assert_eq!(entry_place(None, &MapAreas::default(), &[], &[]), None);
}

#[test]
fn joining_spawns_nothing_until_a_character_enters_and_it_carries_that_character() {
    let mut app = crawler_app();
    app.world_mut()
        .write_message(NetClientJoined { client_id: 3 });
    tick(&mut app);
    ok(&mut app, "assert.spawn_count 0");
    enter(&mut app, 3);
    ok(&mut app, "assert.spawn_count 1");
    let entity = body(&mut app);
    assert_eq!(app.world().get::<PlayerId>(entity), Some(&PlayerId(3)));
    assert_eq!(app.world().get::<PlayingCharacter>(entity), Some(&bob()));
    assert_eq!(
        app.world().get::<Name>(entity).unwrap().as_str(),
        "player_3"
    );
}

#[test]
fn logging_out_saves_the_place_and_the_next_entry_starts_there() {
    let mut app = crawler_app();
    enter(&mut app, 3);
    let entity = body(&mut app);
    let size = *app.world().resource::<CharacterSize>();
    // The character walked somewhere and turned.
    app.world_mut()
        .get_mut::<Transform>(entity)
        .unwrap()
        .translation = size.center(Vec3::new(7.0, 0.0, -3.0));
    app.world_mut().get_mut::<LookAngles>(entity).unwrap().yaw = 2.0;
    app.world_mut().write_message(CharacterLeft {
        player: PlayerId(3),
        character: bob().id,
    });
    tick(&mut app);
    assert!(app.world().get::<RemovePhysicsBody>(entity).is_some());
    assert_eq!(
        saved(&app),
        Some(json!({"feet": [7.0, 0.0, -3.0], "yaw": 2.0}))
    );
    // Back in (as another connection: the old body keeps its name for the rollback window).
    enter(&mut app, 4);
    let again = body(&mut app);
    let transform = app.world().get::<Transform>(again).unwrap();
    assert_eq!(size.feet(transform.translation), Vec3::new(7.0, 0.0, -3.0));
    assert_eq!(app.world().get::<LookAngles>(again).unwrap().yaw, 2.0);
}

#[test]
fn a_disconnect_saves_and_removes_at_once_but_a_takeover_keeps_the_body() {
    let mut app = crawler_app();
    enter(&mut app, 3);
    let entity = body(&mut app);
    // A takeover: connection 3 closed because 5 logged in as the same player, in one tick.
    app.world_mut()
        .write_message(NetClientLeft { client_id: 3 });
    app.world_mut().write_message(NetClientRejoined {
        client_id: 5,
        previous_client_id: 3,
    });
    tick(&mut app);
    assert_eq!(body(&mut app), entity, "the same body");
    assert_eq!(app.world().get::<PlayerId>(entity), Some(&PlayerId(5)));
    assert_eq!(saved(&app), None);
    // A plain disconnect: saved and gone.
    app.world_mut()
        .write_message(NetClientLeft { client_id: 5 });
    tick(&mut app);
    assert!(app.world().get::<RemovePhysicsBody>(entity).is_some());
    assert!(saved(&app).is_some());
}

#[test]
fn played_characters_are_saved_every_autosave_period() {
    let mut app = crawler_app();
    app.insert_resource(PlayerSpawnConfig {
        spawn_on_join: false,
        autosave_seconds: 0.25,
    });
    enter(&mut app, 3);
    assert_eq!(saved(&app), None);
    // 0.25 s at Bevy's default fixed step of 1/64 s is 16 ticks.
    for _ in 0..16 {
        tick(&mut app);
    }
    assert!(saved(&app).is_some());
}
