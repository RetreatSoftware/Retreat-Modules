//! The rifle with only Bevy and the contract crates: casts are faked. Real shots, lag compensation
//! and packet loss are checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_rifle::*;

/// What the faked lag-compensated cast was last asked.
#[derive(Resource, Default)]
struct LastShot {
    view_tick: f64,
    excluded: usize,
}

/// Hits the character of player 2 unless it is excluded.
fn hits_player_two(world: &mut World, cast: &RayCast, view_tick: f64) -> Option<CastHit> {
    *world.resource_mut::<LastShot>() = LastShot {
        view_tick,
        excluded: cast.exclude.len(),
    };
    let mut players = world.query::<(Entity, &PlayerId)>();
    let (entity, _) = players.iter(world).find(|(_, player)| player.0 == 2)?;
    (!cast.exclude.contains(&entity)).then(|| CastHit {
        entity,
        name: "player_2".to_string(),
        distance: 5.0,
        point: Vec3::ZERO,
        normal: Vec3::Z,
    })
}

fn no_shape(_: &mut World, _: &ShapeCast, _: f64) -> Option<CastHit> {
    None
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<InputActions>()
        .init_resource::<LastShot>()
        .insert_resource(LagCaster {
            ray: hits_player_two,
            shape: no_shape,
        })
        .add_plugins(RifleModulePlugin);
    for id in [1, 2] {
        app.world_mut().spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            Health::full(100),
            LookAngles::default(),
            Transform::from_xyz(0.0, 0.9, 0.0),
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

fn requests(app: &mut App) -> Vec<DamageRequested> {
    app.world_mut()
        .resource_mut::<Messages<DamageRequested>>()
        .drain()
        .collect()
}

#[test]
fn a_shot_uses_a_round_and_asks_for_damage_at_the_shooters_view_tick() {
    let mut app = app();
    ok(&mut app, "rifle.fire 0 0 41.5");
    ok(&mut app, "assert.rifle_ammo 1 29");
    assert_eq!(app.world().resource::<LastShot>().view_tick, 41.5);
    assert_eq!(
        requests(&mut app),
        [DamageRequested {
            target: PlayerId(2),
            amount: 20,
            attacker: Some(PlayerId(1)),
            cause: DamageCause::Hitscan,
        }]
    );
    assert_eq!(
        app.world().resource::<Messages<PlayerFired>>().len(),
        1,
        "spawn protection ends"
    );
}

#[test]
fn a_client_predicts_ammo_but_leaves_the_hit_to_the_server() {
    let mut app = app();
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    ok(&mut app, "rifle.fire 0 0");
    ok(&mut app, "assert.rifle_ammo 1 29");
    assert!(requests(&mut app).is_empty());
}

#[test]
fn the_fire_rate_holds_and_the_magazine_reloads_by_itself() {
    let mut app = app();
    let interval = 10; // 0.15 s at 64 ticks per second, rounded up.
    ok(&mut app, "rifle.fire 0 0");
    assert!(run(&mut app, "rifle.fire 0 0").is_err(), "too soon");
    for _ in 1..30 {
        ticks(&mut app, interval);
        ok(&mut app, "rifle.fire 0 0");
    }
    ok(&mut app, "assert.rifle_ammo 1 0");
    ticks(&mut app, interval);
    assert!(run(&mut app, "rifle.fire 0 0").is_err(), "empty");
    ticks(&mut app, 96);
    ok(&mut app, "assert.rifle_ammo 1 30");
}

#[test]
fn early_network_inputs_are_tolerated_without_raising_the_rate() {
    let ticks = RifleTicks {
        fire_interval: 10,
        reload: 96,
    };
    let mut weapon = WeaponStatus {
        ammo: 30,
        magazine: 30,
        reload_ticks_left: 0,
        hits_landed: 0,
    };
    let mut rifle = RifleState::default();
    // A greedy client that tries every tick for 1000 ticks.
    let mut shots = 0;
    for _ in 0..1000 {
        weapon.ammo = 30;
        shots += u32::from(try_fire(&mut weapon, &mut rifle, ticks));
        rifle.cooldown_ticks = rifle.cooldown_ticks.saturating_sub(1);
    }
    assert!((100..=101).contains(&shots), "{shots}");
}

#[test]
fn the_dead_neither_shoot_nor_are_hit() {
    let mut app = app();
    kill(&mut app, 2);
    ok(&mut app, "rifle.fire 0 0");
    assert!(requests(&mut app).is_empty());
    assert_eq!(app.world().resource::<LastShot>().excluded, 2);
    kill(&mut app, 1);
    ticks(&mut app, 20);
    assert!(run(&mut app, "rifle.fire 0 0").is_err());
    // A network input from the dead is silently ignored.
    app.world_mut().insert_resource(NetCommandSender(Some(1)));
    ok(&mut app, "rifle.fire 0 0");
    app.world_mut().insert_resource(NetCommandSender(None));
    ok(&mut app, "assert.rifle_ammo 1 29");
}

fn kill(app: &mut App, id: u32) {
    let world = app.world_mut();
    for (player, mut health) in world.query::<(&PlayerId, &mut Health)>().iter_mut(world) {
        if player.0 == id {
            health.apply_damage(100);
        }
    }
}

#[test]
fn a_finished_match_stops_the_shooting() {
    let mut app = app();
    app.world_mut().spawn(MatchState {
        phase: MatchPhase::Finished {
            winner: None,
            ticks_until_restart: 10,
        },
    });
    assert!(run(&mut app, "rifle.fire 0 0").is_err());
}

#[test]
fn real_damage_counts_as_a_hit_and_a_respawn_refills() {
    let mut app = app();
    ok(&mut app, "rifle.fire 0 0");
    app.world_mut().write_message(DamageApplied {
        target: PlayerId(2),
        amount: 20,
        attacker: Some(PlayerId(1)),
        cause: DamageCause::Hitscan,
        remaining: 80,
    });
    ticks(&mut app, 1);
    ok(&mut app, "assert.rifle_hits 1 1");
    app.world_mut().write_message(PlayerRespawned {
        player: PlayerId(1),
        spawn: SpawnPoint {
            feet: Vec3::ZERO,
            yaw: 0.0,
        },
    });
    ticks(&mut app, 1);
    ok(&mut app, "assert.rifle_ammo 1 30");
    ok(&mut app, "assert.rifle_hits 1 1");
}

#[test]
fn reload_refills_a_part_used_magazine() {
    let mut app = app();
    assert!(run(&mut app, "rifle.reload").is_err(), "full");
    ok(&mut app, "rifle.fire 0 0");
    ok(&mut app, "rifle.reload");
    ticks(&mut app, 96);
    ok(&mut app, "assert.rifle_ammo 1 30");
}

#[test]
fn the_held_button_fires_automatically_at_the_fire_rate() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::MouseLeft);
    ticks(&mut app, 100);
    // Shots in ticks 0, 10, .. 90 after the button went down.
    ok(&mut app, "assert.rifle_ammo 1 20");
}

#[test]
fn a_network_client_queues_its_shots_one_tick_ahead_at_the_same_rate() {
    let mut app = app();
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    app.world_mut().resource_mut::<NetInputQueue>().accepting = true;
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::MouseLeft);
    let mut sent = Vec::new();
    for tick in 0..50 {
        // As the engine does: queued lines run at the start of the tick.
        let lines = std::mem::take(&mut app.world_mut().resource_mut::<NetInputQueue>().lines);
        for line in lines {
            assert!(line.starts_with("rifle.fire "), "{line}");
            ok(&mut app, &line);
            sent.push(tick);
        }
        ticks(&mut app, 1);
    }
    assert_eq!(sent, [1, 11, 21, 31, 41]);
    ok(&mut app, "assert.rifle_ammo 1 25");
}

#[test]
fn aim_angles_point_from_the_eyes_to_the_target() {
    let eye = Vec3::new(0.0, 1.6, 0.0);
    let ahead = aim_angles(eye, Vec3::new(0.0, 1.6, -10.0)).unwrap();
    assert!(ahead.yaw.abs() < 1.0e-6 && ahead.pitch.abs() < 1.0e-6);
    for target in [Vec3::new(4.0, 0.0, 3.0), Vec3::new(-2.0, 5.0, -1.0)] {
        let aim = aim_angles(eye, target).unwrap();
        let wanted = (target - eye).normalize();
        assert!(view_direction(aim).abs_diff_eq(wanted, 1.0e-5));
    }
    assert_eq!(aim_angles(eye, eye), None);
}
