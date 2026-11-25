//! Taking damage and dying.

use crate::harness::*;
use bevy::prelude::*;
use retreat_gameplay_api::*;
use retreat_module_health::{HealthConfig, HealthModulePlugin, RespawnTimer};

#[test]
fn damage_is_applied_reported_and_kills_once() {
    let mut app = app_with_ready_players(2);
    ok(&mut app, "health.damage 1 60 2");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 40");
    assert_eq!(
        drain::<DamageApplied>(&mut app),
        [DamageApplied {
            target: PlayerId(1),
            amount: 60,
            attacker: Some(PlayerId(2)),
            cause: DamageCause::Hitscan,
            remaining: 40,
        }]
    );
    // Two hits in one tick: the second finds nobody alive.
    ok(&mut app, "health.damage 1 100 2");
    ok(&mut app, "health.damage 1 100 2");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health_dead 1");
    assert_eq!(drain::<DamageApplied>(&mut app).len(), 1);
    assert_eq!(
        drain::<PlayerDied>(&mut app),
        [PlayerDied {
            victim: PlayerId(1),
            killer: Some(PlayerId(2)),
            cause: DamageCause::Hitscan,
        }]
    );
    ok(&mut app, "assert.health 2 100");
}

#[test]
fn a_new_player_is_protected_for_one_second() {
    let mut app = app();
    ok(&mut app, "health.add_dummy 1");
    ticks(&mut app, 1);
    ticks(&mut app, PROTECTION_TICKS - 1);
    ok(&mut app, "health.damage 1 10");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 90");

    let mut early = self::app();
    ok(&mut early, "health.add_dummy 1");
    ticks(&mut early, PROTECTION_TICKS - 1);
    ok(&mut early, "health.damage 1 10");
    ticks(&mut early, 1);
    ok(&mut early, "assert.health 1 100");
    assert!(drain::<DamageApplied>(&mut early).is_empty());
}

#[test]
fn kill_ignores_protection() {
    let mut app = app();
    ok(&mut app, "health.add_dummy 1");
    ticks(&mut app, 1);
    ok(&mut app, "health.kill 1");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health_dead 1");
    let died = drain::<PlayerDied>(&mut app);
    assert_eq!(died.len(), 1);
    assert_eq!((died[0].killer, died[0].cause), (None, DamageCause::World));
    let world = app.world_mut();
    assert_eq!(world.query::<&RespawnTimer>().iter(world).count(), 1);
}

#[test]
fn nothing_hurts_while_a_finished_match_shows_its_result() {
    let mut app = app_with_ready_players(1);
    // As the match module would; this module only reads it.
    app.world_mut().spawn((
        Name::new(MATCH_ENTITY_NAME),
        MatchState {
            phase: MatchPhase::Finished {
                winner: None,
                ticks_until_restart: 100,
            },
        },
    ));
    ok(&mut app, "health.damage 1 100");
    ok(&mut app, "health.kill 1");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 100");
}

#[test]
fn falling_out_of_the_map_kills() {
    let mut app = app_with_ready_players(1);
    let world = app.world_mut();
    let mut query = world.query::<&mut Transform>();
    query.single_mut(world).unwrap().translation.y = -11.0;
    ticks(&mut app, 1);
    ok(&mut app, "assert.health_dead 1");
}

#[test]
fn a_game_changes_the_numbers_with_the_config() {
    let mut app = app_with(HealthConfig {
        max_health: 30,
        spawn_protection_seconds: 0.0,
        ..default()
    });
    ok(&mut app, "health.add_dummy 1");
    ticks(&mut app, 1);
    ok(&mut app, "health.damage 1 10");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 20");
}

#[test]
#[should_panic(expected = "max_health")]
fn an_invalid_config_is_refused_at_startup() {
    App::new()
        .insert_resource(HealthConfig {
            max_health: 0,
            ..default()
        })
        .add_plugins(HealthModulePlugin);
}

#[test]
fn commands_name_what_is_wrong() {
    let mut app = app_with_ready_players(1);
    for (line, expected) in [
        ("health.damage 9 1", "no player 9"),
        ("health.damage 1", "usage: health.damage"),
        ("health.add_dummy 1", "already exists"),
        ("assert.health 1 5", "has 100 hit points, expected 5"),
        ("assert.health_dead 1", "is alive"),
    ] {
        let error = run(&mut app, line).expect_err(line).to_string();
        assert!(error.contains(expected), "{line}: {error}");
    }
}

#[test]
fn there_are_at_most_the_maximum_of_players() {
    let mut app = app();
    for id in 0..MAX_PLAYERS {
        ok(&mut app, &format!("health.add_dummy {id}"));
    }
    let error = run(&mut app, "health.add_dummy 100")
        .expect_err("full")
        .to_string();
    assert!(error.contains("already 8 players"), "{error}");
}
