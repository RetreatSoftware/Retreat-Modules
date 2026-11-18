//! The orbit module with only Bevy and the contract crate: commands, held input moving the camera
//! per fixed tick, and errors. The engine-side picture is `scripts/verify/module_orbit_camera.cmds`
//! in Retreat-Engine-3D.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use retreat_engine_api::{CommandRegistry, GameCamera, InputActions, InputKey, run_command_line};
use retreat_module_orbit_camera::{ORBIT_DEGREES_PER_SECOND, Orbit, OrbitCameraModulePlugin};

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        // Like a command script: every update is exactly one fixed tick (the first runs none).
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
        .add_plugins(OrbitCameraModulePlugin);
    app.world_mut().spawn((
        Name::new("cam"),
        GameCamera::looking_at(Vec3::new(0.0, 4.0, 8.0), Vec3::ZERO),
    ));
    app
}

fn camera(app: &mut App) -> GameCamera {
    let world = app.world_mut();
    let mut query = world.query::<&GameCamera>();
    query.single(world).expect("one camera").clone()
}

fn angle(app: &App) -> f32 {
    app.world()
        .resource::<Orbit>()
        .0
        .as_ref()
        .expect("an orbit is running")
        .angle_degrees
}

#[test]
fn start_places_the_camera_on_plus_z_around_the_cameras_target() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "orbit.start cam 6 3").unwrap();
    let placed = camera(&mut app);
    assert!(placed.position.distance(Vec3::new(0.0, 3.0, 6.0)) < 1e-5);
    assert_eq!(placed.target, Vec3::ZERO, "the camera keeps its target");
}

#[test]
fn set_angle_moves_the_camera_and_stop_ends_the_orbit() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "orbit.start cam 6 3").unwrap();
    run_command_line(app.world_mut(), "orbit.set_angle 90").unwrap();
    assert!(camera(&mut app).position.distance(Vec3::new(6.0, 3.0, 0.0)) < 1e-5);
    run_command_line(app.world_mut(), "orbit.stop").unwrap();
    assert_eq!(app.world().resource::<Orbit>().0, None);
}

#[test]
fn a_held_key_turns_the_camera_a_fixed_amount_per_tick() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "orbit.start cam 6 3").unwrap();
    app.update(); // the first update starts the clock and runs no tick
    // The engine recomputes held state each tick from real input; here it is set directly.
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::ArrowRight);
    for _ in 0..64 {
        app.update();
    }
    // 64 ticks = one second of fixed time.
    assert!(
        (angle(&app) - ORBIT_DEGREES_PER_SECOND).abs() < 1e-3,
        "{}",
        angle(&app)
    );
    assert!(camera(&mut app).position.distance(Vec3::new(6.0, 3.0, 0.0)) < 1e-3);

    // The left key turns back; both held cancel out.
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::ArrowLeft);
    for _ in 0..64 {
        app.update();
    }
    assert!(angle(&app).abs() < 1e-3, "{}", angle(&app));
}

#[test]
fn nothing_moves_when_no_key_is_held() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "orbit.start cam 6 3").unwrap();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(angle(&app), 0.0);
}

#[test]
fn errors_name_the_command_the_bad_value_and_the_fix() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "orbit.start nobody 6 3").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("no game camera is named `nobody`"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "orbit.start cam 0 3").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("<radius> must be greater than 0"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "orbit.set_angle 10").unwrap_err();
    assert!(error.to_string().contains("no orbit is running"), "{error}");
    let error = run_command_line(app.world_mut(), "orbit.start cam 6").unwrap_err();
    assert!(error.to_string().contains("usage: orbit.start"), "{error}");
}

#[test]
fn the_actions_are_bound_to_the_arrow_keys_and_run_no_command() {
    let app = module_app();
    let actions = app.world().resource::<InputActions>();
    assert_eq!(actions.keys("orbit_left"), vec![InputKey::ArrowLeft]);
    assert_eq!(actions.keys("orbit_right"), vec![InputKey::ArrowRight]);
    assert_eq!(actions.command_line("orbit_left"), None);
}
