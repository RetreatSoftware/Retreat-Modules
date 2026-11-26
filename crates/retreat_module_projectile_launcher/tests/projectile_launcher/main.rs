//! The launcher with only Bevy and the contract crates: casts are faked by one wall across -Z.
//! Real flights, prediction and packet loss are checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_projectile_launcher::*;

/// A wall across the world at this z, if there is one.
#[derive(Resource, Default)]
struct Wall(Option<f32>);

fn wall_hit(world: &mut World, distance: f32, point: Vec3) -> CastHit {
    CastHit {
        entity: world.spawn_empty().id(),
        name: "wall".to_string(),
        distance,
        point,
        normal: Vec3::Z,
    }
}

fn shape(world: &mut World, cast: &ShapeCast) -> Option<CastHit> {
    let wall = world.resource::<Wall>().0?;
    let PhysicsShape::Ball { radius } = cast.shape else {
        return None;
    };
    let distance = cast.origin.z - radius - wall;
    (cast.direction.z < 0.0 && (0.0..=cast.max_distance).contains(&(distance / -cast.direction.z)))
        .then(|| wall_hit(world, distance / -cast.direction.z, cast.origin))
}

fn ray(world: &mut World, cast: &RayCast) -> Option<CastHit> {
    let wall = world.resource::<Wall>().0?;
    let end = cast.origin + cast.direction * cast.max_distance;
    ((cast.origin.z - wall) * (end.z - wall) < 0.0)
        .then(|| wall_hit(world, (cast.origin.z - wall).abs(), cast.origin))
}

/// Players 1 (at the origin, this machine's) and 2 (`distance_of_two` metres along -Z).
fn app(distance_of_two: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<InputActions>()
        .init_resource::<Wall>()
        .insert_resource(PhysicsCaster { ray, shape })
        .add_plugins(ProjectileLauncherModulePlugin);
    for (id, z) in [(1, 0.0), (2, -distance_of_two)] {
        app.world_mut().spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            Health::full(100),
            LookAngles::default(),
            Transform::from_xyz(0.0, 0.9, z),
        ));
    }
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(1))));
    ticks(&mut app, 1);
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn ticks(app: &mut App, count: u32) {
    for _ in 0..count {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

/// Runs ticks until no rocket flies; returns the damage asked for meanwhile.
fn until_exploded(app: &mut App) -> Vec<DamageRequested> {
    let mut requests = Vec::new();
    for _ in 0..400 {
        ticks(app, 1);
        requests.extend(
            app.world_mut()
                .resource_mut::<Messages<DamageRequested>>()
                .drain(),
        );
        if run(app, "assert.launcher_rockets 0").is_ok() {
            return requests;
        }
    }
    panic!("the rocket never exploded");
}

fn rockets(app: &mut App) -> Vec<(Rocket, NetPose)> {
    let mut query = app.world_mut().query::<(&Rocket, &NetPose)>();
    query
        .iter(app.world())
        .map(|(rocket, pose)| (*rocket, *pose))
        .collect()
}

fn effects(app: &mut App) -> Vec<VisualEffect> {
    std::mem::take(&mut app.world_mut().resource_mut::<VisualEffects>().requests)
}

#[test]
fn firing_uses_a_rocket_and_creates_it_at_the_eyes() {
    let mut app = app(30.0);
    ok(&mut app, "launcher.fire 0 0");
    ok(&mut app, "assert.launcher_ammo 1 3");
    let created = rockets(&mut app);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].0.owner, PlayerId(1));
    assert!(created[0].1.translation.abs_diff_eq(Vec3::Y * 1.6, 1.0e-5));
    assert_eq!(app.world().resource::<Messages<PlayerFired>>().len(), 1);
    assert!(run(&mut app, "launcher.fire 0 0").is_err(), "too soon");
}

#[test]
fn a_rocket_flies_explodes_on_a_wall_once_and_is_removed() {
    let mut app = app(30.0);
    app.world_mut().resource_mut::<Wall>().0 = Some(-10.0);
    ok(&mut app, "launcher.fire 0 0");
    ticks(&mut app, 10);
    let (_, pose) = rockets(&mut app)[0];
    assert!((pose.translation.z + 25.0 * 10.0 / 64.0).abs() < 1.0e-3);
    ok(&mut app, "assert.launcher_rockets 1");
    until_exploded(&mut app);
    let shown = effects(&mut app);
    assert_eq!(shown.len(), blast_look(3.0).len());
    assert_eq!(shown[0].id, "rocket/local/local/0/boom/core");
    assert!((shown[0].position.z + 9.85).abs() < 1.0e-3);
    ticks(&mut app, LINGER_TICKS + 1);
    assert!(effects(&mut app).is_empty());
    let mut gone = app
        .world_mut()
        .query_filtered::<&Rocket, With<RemoveNetEntity>>();
    assert_eq!(gone.iter(app.world()).count(), 1);
}

#[test]
fn the_blast_falls_off_and_hurts_the_shooter_less() {
    // Player 2 stands 1.5 m in front of the wall, player 1 about 1.4 m from the explosion.
    let mut app = app(1.5);
    app.world_mut().resource_mut::<Wall>().0 = Some(-3.0);
    // Down at the ground next to nobody: aim past player 2 (the fake casts hit only the wall).
    ok(&mut app, "launcher.fire 0 0");
    let requests = until_exploded(&mut app);
    let of = |id: u32| {
        requests
            .iter()
            .find(|request| request.target == PlayerId(id))
            .copied()
            .unwrap_or_else(|| panic!("player {id} was not hurt: {requests:?}"))
    };
    let config = LauncherConfig::default();
    let at = Vec3::new(0.0, 1.6, -2.85);
    let size = CharacterSize::default();
    let near = blast_damage(
        &config,
        distance_to_body(at, Vec3::new(0.0, 0.9, -1.5), size),
    );
    let far = blast_damage(
        &config,
        distance_to_body(at, Vec3::new(0.0, 0.9, 0.0), size),
    );
    assert_eq!((of(2).amount, of(2).cause), (near, DamageCause::Blast));
    assert_eq!((of(1).amount, of(1).attacker), (far / 2, Some(PlayerId(1))));
    assert!(far < near && near < config.damage);
}

#[test]
fn a_wall_between_shields_from_the_blast() {
    let mut app = app(4.0);
    app.world_mut().resource_mut::<Wall>().0 = Some(-3.0);
    ok(&mut app, "launcher.fire 0 0");
    let requests = until_exploded(&mut app);
    assert!(requests.iter().all(|request| request.target != PlayerId(2)));
}

#[test]
fn the_dead_are_not_hurt_and_do_not_shoot() {
    let mut app = app(1.5);
    app.world_mut().resource_mut::<Wall>().0 = Some(-3.0);
    let mut players = app.world_mut().query::<(&PlayerId, &mut Health)>();
    for (player, mut health) in players.iter_mut(app.world_mut()) {
        if player.0 == 2 {
            health.apply_damage(100);
        }
    }
    ok(&mut app, "launcher.fire 0 0");
    let requests = until_exploded(&mut app);
    assert!(requests.iter().all(|request| request.target == PlayerId(1)));
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(2))));
    assert!(run(&mut app, "launcher.fire 0 0").is_err());
}

#[test]
fn a_rocket_that_hits_nothing_explodes_at_the_end_of_its_life() {
    let mut app = app(30.0);
    ok(&mut app, "launcher.fire 0 1.2");
    ticks(&mut app, 255);
    ok(&mut app, "assert.launcher_rockets 1");
    ticks(&mut app, 2);
    ok(&mut app, "assert.launcher_rockets 0");
}

#[test]
fn a_client_predicts_the_rocket_and_leaves_the_damage_to_the_server() {
    let mut app = app(1.5);
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    app.world_mut().resource_mut::<Wall>().0 = Some(-3.0);
    ok(&mut app, "launcher.fire 0 0");
    assert!(until_exploded(&mut app).is_empty());
    assert_eq!(effects(&mut app)[0].time, EffectTime::Present);
}

#[test]
fn the_own_rocket_is_smoothed_and_anothers_is_drawn_in_the_past() {
    let mut app = app(30.0);
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    ok(&mut app, "launcher.fire 0 0");
    let other = app
        .world_mut()
        .spawn((
            Name::new("rocket/2/5/0"),
            Rocket {
                owner: PlayerId(2),
                direction: Vec3::X,
                age: 0,
                exploded: 0,
            },
            NetPose::at(Vec3::Y),
        ))
        .id();
    app.world_mut().run_schedule(Update);
    assert!(app.world().get::<RemoteInterpolation>(other).is_some());
    assert!(app.world().get::<SphereVisual>(other).is_some());
    let mut own = app
        .world_mut()
        .query_filtered::<&Rocket, (With<RenderSmoothing>, Without<RemoteInterpolation>)>();
    assert_eq!(own.iter(app.world()).count(), 1);
}

#[test]
fn the_held_button_fires_at_the_fire_rate_and_an_empty_magazine_reloads() {
    let mut app = app(30.0);
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::MouseRight);
    // Shots in ticks 0, 58, 116 and 174 (0.9 s at 64 ticks per second, rounded up).
    ticks(&mut app, 175);
    ok(&mut app, "assert.launcher_ammo 1 0");
    ticks(&mut app, 129);
    ok(&mut app, "assert.launcher_ammo 1 3");
}

#[test]
fn the_blast_numbers_are_a_straight_line() {
    let config = LauncherConfig::default();
    assert_eq!(blast_damage(&config, 0.0), 80);
    assert_eq!(blast_damage(&config, 1.5), 50);
    assert_eq!(blast_damage(&config, 3.0), 20);
    assert_eq!(blast_damage(&config, 3.01), 0);
    let size = CharacterSize::default();
    assert_eq!(
        distance_to_body(Vec3::new(0.2, 0.9, 0.0), Vec3::Y * 0.9, size),
        0.0
    );
    let above = distance_to_body(Vec3::Y * 2.8, Vec3::Y * 0.9, size);
    assert!((above - 1.0).abs() < 1.0e-5, "{above}");
}

#[test]
fn a_new_match_removes_the_rockets_in_flight() {
    let mut app = app(30.0);
    ok(&mut app, "launcher.fire 0 0");
    ticks(&mut app, 3);
    app.world_mut().write_message(MatchRestarted);
    ticks(&mut app, 1);
    let mut gone = app
        .world_mut()
        .query_filtered::<&Rocket, With<RemoveNetEntity>>();
    assert_eq!(gone.iter(app.world()).count(), 1);
}

#[test]
fn an_explosion_stays_inside_its_blast_radius() {
    for part in blast_look(3.0) {
        assert!(part.offset.length() + part.radius <= 3.0, "{part:?}");
    }
}

#[test]
fn the_launch_and_the_explosion_are_heard_where_they_happen() {
    let mut app = app(30.0);
    {
        let mut config = app.world_mut().resource_mut::<LauncherConfig>();
        config.launch_sound = Some(SoundCue::new("sounds/launch.wav", 0.5));
        config.explosion_sound = Some(SoundCue::new("sounds/boom.wav", 0.5));
    }
    app.world_mut().resource_mut::<Wall>().0 = Some(-10.0);
    ok(&mut app, "launcher.fire 0 0");
    until_exploded(&mut app);
    let mut heard: Vec<(String, f32)> =
        std::mem::take(&mut app.world_mut().resource_mut::<SoundEvents>().requests)
            .into_iter()
            .map(|event| (event.id, event.at.unwrap().position.z))
            .collect();
    heard.dedup_by(|a, b| a.0 == b.0);
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert_eq!(heard[0].0, "rocket/local/local/0/launch");
    assert!(
        heard[1].0.ends_with("/boom") && heard[1].1 < -9.0,
        "{heard:?}"
    );
}
