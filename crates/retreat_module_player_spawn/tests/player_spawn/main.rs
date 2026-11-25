//! Player spawning with only Bevy and the contract crates. With the real engine (bodies, network)
//! it is checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, NetClientJoined, NetClientLeft, PhysicsBody, PhysicsBodyKind,
    RemovePhysicsBody, run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_player_spawn::PlayerSpawnModulePlugin;

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
