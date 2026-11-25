//! The camera with only Bevy and the contract crates (casts are faked). Real walls are checked in
//! the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::{
    CastHit, CommandRegistry, CommandResult, GameCamera, PhysicsCaster, RayCast, ShapeCast,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_third_person_camera::{
    ShoulderConfig, ThirdPersonCameraModulePlugin, shoulder_points,
};

fn nothing(_: &mut World, _: &ShapeCast) -> Option<CastHit> {
    None
}

/// A wall 1 m behind the head (at z 1).
fn wall_behind(_: &mut World, cast: &ShapeCast) -> Option<CastHit> {
    let travel = (1.0 - cast.origin.z) / cast.direction.normalize().z;
    (travel >= 0.0 && travel <= cast.max_distance).then(|| CastHit {
        entity: Entity::PLACEHOLDER,
        name: "wall".to_string(),
        distance: travel,
        point: Vec3::ZERO,
        normal: Vec3::NEG_Z,
    })
}

fn no_ray(_: &mut World, _: &RayCast) -> Option<CastHit> {
    None
}

fn app(shape: fn(&mut World, &ShapeCast) -> Option<CastHit>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(PhysicsCaster { ray: no_ray, shape })
        .add_plugins(ThirdPersonCameraModulePlugin);
    app.world_mut().spawn((
        Name::new("view"),
        GameCamera::looking_at(Vec3::new(0.0, 10.0, 10.0), Vec3::ZERO),
    ));
    app.world_mut().spawn((
        PlayerId(0),
        LookAngles::default(),
        Transform::from_xyz(0.0, 0.9, 0.0),
    ));
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn camera(app: &mut App) -> GameCamera {
    app.world_mut().run_schedule(FixedPostUpdate);
    let world = app.world_mut();
    world.query::<&GameCamera>().single(world).unwrap().clone()
}

#[test]
fn the_camera_sits_behind_the_right_shoulder_and_can_swap() {
    let mut app = app(nothing);
    ok(&mut app, "shoulder.follow view 0");
    let placed = camera(&mut app);
    assert!(
        placed
            .position
            .abs_diff_eq(Vec3::new(0.6, 1.5, 3.5), 1.0e-5),
        "{placed:?}"
    );
    assert!((placed.target - placed.position).abs_diff_eq(Vec3::NEG_Z, 1.0e-5));
    ok(&mut app, "shoulder.swap");
    assert!((camera(&mut app).position.x + 0.6).abs() < 1.0e-5);
}

#[test]
fn a_wall_pulls_the_camera_in() {
    let mut app = app(wall_behind);
    ok(&mut app, "shoulder.follow view 0");
    let placed = camera(&mut app);
    assert!((placed.position.z - 1.0).abs() < 1.0e-5, "{placed:?}");
    ok(&mut app, "assert.shoulder_distance view 1 1.3");
}

#[test]
fn without_a_player_argument_it_waits_for_the_local_player() {
    let mut app = app(nothing);
    ok(&mut app, "shoulder.follow view");
    assert_eq!(camera(&mut app).position, Vec3::new(0.0, 10.0, 10.0));
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    assert!((camera(&mut app).position.z - 3.5).abs() < 1.0e-5);
    ok(&mut app, "shoulder.stop view");
    app.world_mut().insert_resource(SoloPlayer(None));
}

#[test]
fn looking_up_lowers_the_camera() {
    let look = LookAngles {
        yaw: 0.0,
        pitch: 0.5,
    };
    let points = shoulder_points(
        Vec3::new(0.0, 0.9, 0.0),
        look,
        1.0,
        &ShoulderConfig::default(),
    );
    assert_eq!(points.pivot, Vec3::new(0.0, 1.5, 0.0));
    assert!(points.camera.y < points.pivot.y);
    assert!(points.camera.z > 0.0);
}

#[test]
fn errors_name_the_missing_camera() {
    let mut app = app(nothing);
    let error = run(&mut app, "shoulder.follow other")
        .expect_err("unknown")
        .to_string();
    assert!(error.contains("no camera named `other`"), "{error}");
}
