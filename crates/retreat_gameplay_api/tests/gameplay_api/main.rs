//! The shared rules of the gameplay contract.

use bevy::prelude::*;
use retreat_gameplay_api::*;

#[test]
fn damage_removes_hit_points_and_kills_exactly_once() {
    let mut health = Health::full(100);
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 60,
            killed: false
        }
    );
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 40,
            killed: true
        }
    );
    assert!(!health.is_alive());
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 0,
            killed: false
        }
    );
    health.restore();
    assert_eq!((health.current(), health.max()), (100, 100));
}

#[test]
fn zero_damage_does_nothing_and_nobody_is_born_dead() {
    let mut health = Health::full(0);
    assert!(health.is_alive());
    assert!(!health.apply_damage(0).killed);
}

fn board(lines: &[(u32, i32, u32)]) -> Scoreboard {
    let mut board = Scoreboard::default();
    for &(id, points, deaths) in lines {
        let entry = board.entry_mut(PlayerId(id));
        entry.points = points;
        entry.deaths = deaths;
    }
    board
}

#[test]
fn standings_rank_by_points_then_deaths_then_id() {
    let board = board(&[(3, 2, 0), (1, 5, 4), (2, 5, 1), (0, 2, 0)]);
    let order: Vec<u32> = board.standings().iter().map(|e| e.player.0).collect();
    assert_eq!(order, [2, 1, 0, 3]);
}

#[test]
fn the_winner_needs_strictly_the_most_points() {
    assert_eq!(Scoreboard::default().winner(), None);
    assert_eq!(board(&[(4, -1, 1)]).winner(), Some(PlayerId(4)));
    assert_eq!(board(&[(1, 3, 0), (2, 4, 9)]).winner(), Some(PlayerId(2)));
    assert_eq!(board(&[(1, 3, 0), (2, 3, 2)]).winner(), None);
}

#[test]
fn entries_are_unique_and_removable() {
    let mut board = board(&[(1, 1, 0)]);
    board.entry_mut(PlayerId(1)).kills += 1;
    assert_eq!(board.standings().len(), 1);
    assert_eq!(board.entry(PlayerId(1)).map(|e| e.kills), Some(1));
    board.remove(PlayerId(1));
    assert_eq!(board.entry(PlayerId(1)), None);
}

#[test]
fn only_a_finished_match_stops_combat() {
    let running = MatchState {
        phase: MatchPhase::Running { ticks_left: 1 },
    };
    assert!(running.allows_combat());
    let finished = MatchState {
        phase: MatchPhase::Finished {
            winner: None,
            ticks_until_restart: 1,
        },
    };
    assert!(!finished.allows_combat());
}

#[test]
fn seconds_become_whole_ticks_rounded_up() {
    let tick = std::time::Duration::from_secs_f64(1.0 / 64.0);
    assert_eq!(ticks_from_seconds(3.0, tick), 192);
    assert_eq!(ticks_from_seconds(0.15, tick), 10);
    assert_eq!(ticks_from_seconds(0.001, tick), 1);
    for nothing in [0.0, -1.0, f32::NAN] {
        assert_eq!(ticks_from_seconds(nothing, tick), 0);
    }
}

#[test]
fn the_contract_registers_the_shared_net_state() {
    let mut app = App::new();
    add_gameplay_contract(&mut app);
    let registry = app
        .world()
        .resource::<retreat_engine_api::NetStateRegistry>();
    let ids: Vec<&str> = registry.types().map(|kind| kind.id).collect();
    assert_eq!(
        ids,
        [
            "gameplay.health",
            "gameplay.match",
            "gameplay.player",
            "gameplay.scoreboard"
        ]
    );
}

#[test]
fn players_are_named_from_their_id() {
    assert_eq!(player_entity_name(PlayerId(7)), "player_7");
}

#[test]
fn adding_the_contract_twice_registers_everything_once() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    add_gameplay_contract(&mut app);
    app.world_mut()
        .resource_mut::<SpawnPoints>()
        .0
        .push(SpawnPoint {
            feet: Vec3::ZERO,
            yaw: 0.0,
        });
    add_gameplay_contract(&mut app);
    assert_eq!(app.world().resource::<SpawnPoints>().0.len(), 1);
    app.world_mut().write_message(MatchRestarted);
    app.update();
}
