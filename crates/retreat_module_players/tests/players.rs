//! The players module with only Bevy and the contract crate: join and leave messages, the command with
//! its sender, clamping and errors, and held keys queueing movement lines.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, InputActions, InputKey, NetAllowedCommands, NetClientJoined, NetClientLeft,
    NetCommandSender, NetInputQueue, PhysicsBody, PhysicsVelocityChange, RemovePhysicsBody,
    run_command_line,
};
use retreat_module_players::{
    HELD_MOVE_PER_TICK, MAX_MOVE_PER_COMMAND, PlayersModulePlugin, player_name, spawn_position,
};

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(PlayersModulePlugin);
    app
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn player(app: &mut App, id: u32) -> Option<Entity> {
    let wanted = player_name(id);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find(|(_, name)| name.as_str() == wanted)
        .map(|(entity, _)| entity)
}

#[test]
fn a_joining_client_gets_a_body_at_its_spawn_point_and_a_leaving_client_loses_it() {
    let mut app = module_app();
    app.world_mut()
        .write_message(NetClientJoined { client_id: 2 });
    tick(&mut app);
    let body = player(&mut app, 2).expect("player_2 exists");
    assert!(app.world().entity(body).contains::<PhysicsBody>());
    assert_eq!(
        app.world().get::<Transform>(body).unwrap().translation,
        spawn_position(2)
    );
    assert!(
        player(&mut app, 1).is_none(),
        "only the joining client gets a body"
    );

    app.world_mut()
        .write_message(NetClientLeft { client_id: 2 });
    tick(&mut app);
    // Removal is deferred to the engine: the body asks to be removed instead of being despawned.
    assert!(app.world().entity(body).contains::<RemovePhysicsBody>());
}

#[test]
fn the_move_command_acts_on_the_sender_and_adds_up_within_a_tick() {
    let mut app = module_app();
    app.world_mut()
        .write_message(NetClientJoined { client_id: 0 });
    app.world_mut()
        .write_message(NetClientJoined { client_id: 1 });
    tick(&mut app);
    app.world_mut().insert_resource(NetCommandSender(Some(1)));
    run_command_line(app.world_mut(), "player.move 0.25 0").unwrap();
    run_command_line(app.world_mut(), "player.move 0.125 -0.25").unwrap();
    let mover = player(&mut app, 1).unwrap();
    assert_eq!(
        app.world().get::<PhysicsVelocityChange>(mover).unwrap().0,
        Vec3::new(0.375, 0.0, -0.25)
    );
    let other = player(&mut app, 0).unwrap();
    assert!(app.world().get::<PhysicsVelocityChange>(other).is_none());
}

#[test]
fn one_command_cannot_launch_a_body() {
    let mut app = module_app();
    app.world_mut()
        .write_message(NetClientJoined { client_id: 0 });
    tick(&mut app);
    app.world_mut().insert_resource(NetCommandSender(Some(0)));
    run_command_line(app.world_mut(), "player.move 1000 -1000").unwrap();
    let body = player(&mut app, 0).unwrap();
    assert_eq!(
        app.world().get::<PhysicsVelocityChange>(body).unwrap().0,
        Vec3::new(MAX_MOVE_PER_COMMAND, 0.0, -MAX_MOVE_PER_COMMAND)
    );
}

#[test]
fn errors_name_the_command_the_value_and_the_fix() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "player.move 1 0").unwrap_err();
    assert!(
        error.to_string().contains("moves the player who sent it"),
        "{error}"
    );
    app.world_mut().insert_resource(NetCommandSender(Some(7)));
    let error = run_command_line(app.world_mut(), "player.move 1 0").unwrap_err();
    assert!(
        error.to_string().contains("no body `player_7` yet"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "player.move nan 0").unwrap_err();
    assert!(error.to_string().contains("must be finite"), "{error}");
    let error = run_command_line(app.world_mut(), "player.move 1").unwrap_err();
    assert!(
        error.to_string().contains("usage: player.move <x> <z>"),
        "{error}"
    );
}

#[test]
fn the_command_is_allowed_for_clients() {
    let app = module_app();
    assert!(
        app.world()
            .resource::<NetAllowedCommands>()
            .0
            .contains("player.move")
    );
}

#[test]
fn held_keys_queue_one_move_line_per_tick_only_in_a_network_client() {
    let mut app = module_app();
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::ArrowRight || key == InputKey::ArrowUp);
    // Not a client: lines are dropped.
    tick(&mut app);
    assert!(app.world().resource::<NetInputQueue>().lines.is_empty());
    // A network client: one line per tick.
    app.world_mut().resource_mut::<NetInputQueue>().accepting = true;
    tick(&mut app);
    tick(&mut app);
    let line = format!("player.move {HELD_MOVE_PER_TICK} -{HELD_MOVE_PER_TICK}");
    assert_eq!(
        app.world().resource::<NetInputQueue>().lines,
        [line.clone(), line]
    );
    // No key held: nothing queued.
    app.world_mut()
        .resource_mut::<NetInputQueue>()
        .lines
        .clear();
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|_| false);
    tick(&mut app);
    assert!(app.world().resource::<NetInputQueue>().lines.is_empty());
}
