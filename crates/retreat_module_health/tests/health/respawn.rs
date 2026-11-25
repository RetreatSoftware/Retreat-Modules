//! Coming back.

use crate::harness::*;
use bevy::prelude::*;
use retreat_gameplay_api::*;
use retreat_module_health::choose_spawn_point;

#[test]
fn the_dead_respawn_after_the_delay_at_the_spawn_point_farthest_from_the_living() {
    let mut app = app_with_ready_players(2);
    // Dummies start apart: the first point, then the farthest one from it.
    assert_eq!(position(&mut app, 1), Vec3::new(0.0, 0.0, 20.0));
    assert_eq!(position(&mut app, 2), Vec3::new(0.0, 0.0, -20.0));
    ok(&mut app, "health.kill 1");
    ticks(&mut app, 1);
    drain::<PlayerRespawned>(&mut app);
    ticks(&mut app, RESPAWN_TICKS - 1);
    ok(&mut app, "assert.health_dead 1");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 100");
    let respawned = drain::<PlayerRespawned>(&mut app);
    assert_eq!(respawned.len(), 1);
    assert_eq!(respawned[0].player, PlayerId(1));
    // Player 2 stands on the second point; the first is the farthest.
    assert_eq!(respawned[0].spawn.feet, Vec3::new(0.0, 0.0, 20.0));
    // Protected again.
    ok(&mut app, "health.damage 1 50");
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 100");
}

#[test]
fn a_restart_respawns_everyone_on_different_points() {
    let mut app = app_with_ready_players(3);
    ok(&mut app, "health.kill 2");
    ok(&mut app, "health.damage 1 10");
    ticks(&mut app, 1);
    drain::<PlayerRespawned>(&mut app);
    app.world_mut().write_message(MatchRestarted);
    ticks(&mut app, 1);
    ok(&mut app, "assert.health 1 100");
    ok(&mut app, "assert.health 2 100");
    let respawned = drain::<PlayerRespawned>(&mut app);
    let order: Vec<u32> = respawned.iter().map(|r| r.player.0).collect();
    assert_eq!(order, [1, 2, 3]);
    let points: Vec<Vec3> = (1..=3).map(|id| position(&mut app, id)).collect();
    assert!(points[0] != points[1] && points[1] != points[2] && points[0] != points[2]);
    // The old respawn timer of player 2 is gone: nothing else happens later.
    ticks(&mut app, RESPAWN_TICKS + 1);
    assert!(drain::<PlayerRespawned>(&mut app).is_empty());
}

#[test]
fn without_a_map_players_respawn_where_they_are() {
    let mut app = app_with_ready_players(1);
    app.world_mut().resource_mut::<SpawnPoints>().0.clear();
    let before = position(&mut app, 1);
    ok(&mut app, "health.kill 1");
    ticks(&mut app, RESPAWN_TICKS + 1);
    ok(&mut app, "assert.health 1 100");
    assert_eq!(position(&mut app, 1), before);
    assert_eq!(drain::<PlayerRespawned>(&mut app)[0].spawn.feet, before);
}

#[test]
fn the_spawn_choice_is_the_farthest_point_and_the_first_on_a_tie() {
    let points: Vec<SpawnPoint> = [-10.0, 0.0, 10.0]
        .map(|x| SpawnPoint {
            feet: Vec3::new(x, 0.0, 0.0),
            yaw: 0.0,
        })
        .to_vec();
    assert_eq!(choose_spawn_point(&[], &[]), None);
    assert_eq!(choose_spawn_point(&points, &[]), Some(0));
    assert_eq!(choose_spawn_point(&points, &[points[0].feet]), Some(2));
    assert_eq!(choose_spawn_point(&points, &[Vec3::ZERO]), Some(0));
    let both_ends = [points[0].feet, points[2].feet];
    assert_eq!(choose_spawn_point(&points, &both_ends), Some(1));
}
