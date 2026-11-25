//! Character visuals with only Bevy and the contract crates: the rules, and what the module puts on
//! players. The pictures are checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_character_visuals::*;

fn clip(name: &str, matched_speed: Option<f32>) -> Clip {
    Clip {
        name: name.to_string(),
        rate: 1.0,
        matched_speed,
    }
}

fn fox() -> CharacterModel {
    CharacterModel {
        path: "models/Fox.glb".to_string(),
        scale: 0.012,
        yaw: std::f32::consts::PI,
        idle: clip("Survey", None),
        walk: clip("Walk", Some(3.0)),
        run: clip("Run", Some(9.0)),
        air: clip("Run", None),
    }
}

fn app(config: CharacterVisualsConfig) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(config)
        .add_plugins(CharacterVisualsModulePlugin);
    app
}

fn with_fox() -> App {
    app(CharacterVisualsConfig {
        model: Some(fox()),
        ..default()
    })
}

fn player(app: &mut App, id: u32, velocity: Vec3, grounded: bool) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            PhysicsVelocity {
                linear: velocity,
                angular: Vec3::ZERO,
            },
            CharacterGround {
                grounded,
                normal: Vec3::Y,
            },
        ))
        .id()
}

fn ok(app: &mut App, line: &str) {
    run_command_line(app.world_mut(), line)
        .unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

#[test]
fn what_a_character_does_follows_its_speed_and_ground() {
    let config = CharacterVisualsConfig::default();
    let doing = |x: f32, y: f32, grounded| locomotion_of(Vec3::new(x, y, 0.0), grounded, &config);
    assert_eq!(doing(0.2, 0.0, Some(true)), Locomotion::Idle);
    assert_eq!(doing(6.0, 0.0, Some(true)), Locomotion::Walk);
    assert_eq!(doing(9.0, 0.0, Some(true)), Locomotion::Run);
    assert_eq!(doing(0.0, 0.0, Some(false)), Locomotion::Air);
    // Without a ground readout the vertical speed decides.
    assert_eq!(doing(0.0, 0.1, None), Locomotion::Idle);
    assert_eq!(doing(6.0, -5.0, None), Locomotion::Air);
}

#[test]
fn a_character_faces_where_it_moves_and_keeps_that_when_it_stops() {
    let config = CharacterVisualsConfig::default();
    let quarter = std::f32::consts::FRAC_PI_2;
    assert_eq!(facing_of(Vec3::NEG_Z, 9.0, &config), 0.0);
    assert!((facing_of(Vec3::X, 9.0, &config) + quarter).abs() < 1.0e-6);
    assert!((facing_of(Vec3::NEG_X, 9.0, &config) - quarter).abs() < 1.0e-6);
    assert_eq!(facing_of(Vec3::new(0.0, 5.0, 0.0), 9.0, &config), 9.0);
}

#[test]
fn moving_clips_follow_the_speed() {
    let model = fox();
    let walk = clips_for(&model, Locomotion::Walk, Vec3::X * 6.0);
    assert_eq!(walk[0].clip, "Walk");
    assert_eq!(walk[0].speed, 2.0);
    let idle = clips_for(&model, Locomotion::Idle, Vec3::ZERO);
    assert_eq!((idle[0].clip.as_str(), idle[0].speed), ("Survey", 1.0));
    let fastest = clips_for(&model, Locomotion::Walk, Vec3::X * 100.0);
    assert_eq!(fastest[0].speed, MAX_CLIP_SPEED);
}

#[test]
fn players_get_the_model_a_color_each_and_the_clip_for_what_they_do() {
    let mut app = with_fox();
    let running = player(&mut app, 0, Vec3::NEG_Z * 9.0, true);
    let jumping = player(&mut app, 1, Vec3::Y * 5.0, false);
    app.update();
    ok(&mut app, "assert.visuals_state 0 run");
    ok(&mut app, "assert.visuals_state 1 air");
    ok(&mut app, "assert.visuals_color 0 0");
    ok(&mut app, "assert.visuals_color 1 1");
    ok(&mut app, "assert.visuals_facing 0 0 0.01");
    let visual = app.world().get::<CharacterVisual>(running).unwrap();
    assert_eq!(visual.model.as_deref(), Some("models/Fox.glb"));
    assert_eq!(visual.offset, Vec3::new(0.0, -0.9, 0.0));
    assert_eq!(visual.yaw, std::f32::consts::PI);
    assert_eq!(visual.clips[0].clip, "Run");
    assert_ne!(
        app.world().get::<EntityColor>(running),
        app.world().get::<EntityColor>(jumping)
    );
}

#[test]
fn a_still_character_faces_where_it_looks_until_it_moves() {
    let mut app = app(CharacterVisualsConfig::default());
    let entity = player(&mut app, 0, Vec3::ZERO, true);
    app.world_mut().entity_mut(entity).insert(LookAngles {
        yaw: 1.0,
        pitch: 0.0,
    });
    app.update();
    ok(&mut app, "assert.visuals_facing 0 1 0.001");
    let visual = app.world().get::<CharacterVisual>(entity).unwrap();
    assert_eq!((visual.model.as_ref(), visual.yaw), (None, 1.0));
}

#[test]
fn only_other_players_are_smoothed_and_a_server_smooths_nobody() {
    let mut app = with_fox();
    let first = player(&mut app, 0, Vec3::ZERO, true);
    let second = player(&mut app, 1, Vec3::ZERO, true);
    app.update();
    let smoothed = |app: &App, entity| app.world().get::<RenderSmoothing>(entity).is_some();
    assert!(!smoothed(&app, first) && !smoothed(&app, second));
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    app.update();
    assert!(smoothed(&app, first));
    assert!(!smoothed(&app, second));
}

#[test]
#[should_panic(expected = "colors")]
fn an_invalid_config_is_refused_at_startup() {
    let _ = app(CharacterVisualsConfig {
        colors: Vec::new(),
        ..default()
    });
}
