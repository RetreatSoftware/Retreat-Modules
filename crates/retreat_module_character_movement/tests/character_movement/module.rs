//! The module in an app: commands, local input and what a client sends.

use bevy::prelude::*;
use retreat_engine_api::{
    AnalogInput, CharacterGround, CharacterStepping, CommandRegistry, CommandResult, InputActions,
    NetInputQueue, NetLocalClient, PhysicsBody, PhysicsTeleport, PhysicsVelocitySet,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_character_movement::{
    ADVANCE_AXIS, CharacterMovementModulePlugin, MovementState,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(CharacterMovementModulePlugin);
    app
}

fn character(app: &mut App, id: u32) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            PhysicsBody::character(
                CharacterSize::default().capsule_half_height(),
                CharacterSize::default().radius,
            ),
            // As the engine would report on flat ground.
            CharacterGround {
                grounded: true,
                normal: Vec3::Y,
            },
        ))
        .id()
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

/// One tick, with the engine's input update done by hand.
fn tick(app: &mut App) {
    let world = app.world_mut();
    world.resource_scope(|world, mut analog: Mut<AnalogInput>| {
        let actions = world.resource::<InputActions>();
        analog.update(|action| actions.is_held(action));
    });
    world.run_schedule(FixedUpdate);
}

#[test]
fn the_solo_players_axes_move_its_body() {
    let mut app = app();
    let entity = character(&mut app, 0);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut()
        .resource_mut::<AnalogInput>()
        .force_axis(ADVANCE_AXIS, Some(1.0))
        .unwrap();
    tick(&mut app);
    tick(&mut app);
    let velocity = app.world().get::<PhysicsVelocitySet>(entity).unwrap().0;
    assert!(velocity.z < 0.0, "{velocity}");
}

#[test]
fn the_mouse_turns_the_view_within_the_pitch_limit() {
    let mut app = app();
    let entity = character(&mut app, 0);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    tick(&mut app);
    app.world_mut()
        .resource_mut::<AnalogInput>()
        .add_look(Vec2::new(-100.0, -10_000.0));
    tick(&mut app);
    let look = *app.world().get::<LookAngles>(entity).unwrap();
    assert!((look.yaw - 0.22).abs() < 1.0e-5, "{look:?}");
    assert!(
        (look.pitch - 85.0_f32.to_radians()).abs() < 1.0e-5,
        "{look:?}"
    );
}

#[test]
fn a_client_queues_one_input_line_a_tick_for_its_own_character_only() {
    let mut app = app();
    character(&mut app, 0);
    let mine = character(&mut app, 1);
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    app.world_mut().resource_mut::<NetInputQueue>().accepting = true;
    tick(&mut app);
    let lines = app.world().resource::<NetInputQueue>().lines.clone();
    assert_eq!(lines, ["movement.input 0 0 0 0 0 0 0"]);
    run(&mut app, "movement.input 0.5 1 1.5 -0.25 1 0 1").unwrap();
    let state = *app.world().get::<MovementState>(mine).unwrap();
    assert_eq!((state.input.strafe, state.input.advance), (0.5, 1.0));
    assert!(state.input.sprint && !state.input.jump && state.input.dash);
    assert_eq!(app.world().get::<LookAngles>(mine).unwrap().yaw, 1.5);
}

#[test]
fn a_server_has_no_local_player_and_leaves_inputs_alone() {
    let mut app = app();
    let entity = character(&mut app, 0);
    tick(&mut app);
    app.world_mut()
        .get_mut::<MovementState>(entity)
        .unwrap()
        .input
        .advance = 1.0;
    tick(&mut app);
    let state = app.world().get::<MovementState>(entity).unwrap();
    assert_eq!(state.input.advance, 1.0);
}

#[test]
fn bad_input_is_refused() {
    let mut app = app();
    character(&mut app, 0);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    tick(&mut app);
    for (line, expected) in [
        ("movement.input nan 0 0 0 0 0 0", "finite"),
        ("movement.input 0 0 inf 0 0 0 0", "finite"),
        ("movement.input 0 0 0 0 2 0 0", "0 or 1"),
        ("movement.input 0 0 0", "usage: movement.input"),
        ("assert.movement_near 0 9 9 9 0.1", "stands at"),
        ("assert.movement_grounded 7 true", "no character"),
    ] {
        let error = run(&mut app, line).expect_err(line).to_string();
        assert!(error.contains(expected), "{line}: {error}");
    }
}

#[test]
fn a_character_gets_the_engines_step_handling() {
    let mut app = app();
    let entity = character(&mut app, 0);
    tick(&mut app);
    let stepping = app.world().get::<CharacterStepping>(entity).unwrap();
    assert_eq!(
        (stepping.max_step_height, stepping.max_slope_degrees),
        (0.35, 45.0)
    );
}

#[test]
fn a_respawn_teleports_the_character_and_ends_its_dash() {
    let mut app = app();
    let entity = character(&mut app, 0);
    let other = character(&mut app, 1);
    tick(&mut app);
    app.world_mut()
        .get_mut::<MovementState>(entity)
        .unwrap()
        .dash_cooldown_ticks = 50;
    app.world_mut().write_message(PlayerRespawned {
        player: PlayerId(0),
        spawn: SpawnPoint {
            feet: Vec3::new(3.0, 0.0, 4.0),
            yaw: 1.5,
        },
    });
    tick(&mut app);
    let world = app.world();
    assert_eq!(
        world.get::<PhysicsTeleport>(entity).map(|t| t.0),
        Some(Vec3::new(3.0, 0.9, 4.0))
    );
    assert!(world.get::<PhysicsVelocitySet>(entity).is_none());
    assert_eq!(world.get::<LookAngles>(entity).unwrap().yaw, 1.5);
    let state = world.get::<MovementState>(entity).unwrap();
    assert_eq!(state.dash_cooldown_ticks, 0);
    assert!(world.get::<PhysicsTeleport>(other).is_none());
}

#[test]
fn a_view_can_be_asserted_the_short_way_round() {
    let mut app = app();
    character(&mut app, 0);
    tick(&mut app);
    run(&mut app, "assert.movement_look 0 6.2831 0.01").unwrap();
    let error = run(&mut app, "assert.movement_look 0 1 0.01")
        .expect_err("different")
        .to_string();
    assert!(error.contains("looks towards yaw 0"), "{error}");
}
