//! The match module alone: players are bare `PlayerId` entities and deaths are written by the test,
//! as a health module would. Together with health, and over the network, it is checked in the game
//! (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::{CommandRegistry, CommandResult, run_command_line};
use retreat_gameplay_api::*;
use retreat_module_match::{MatchConfig, MatchModulePlugin};

/// 2 s match and 1 s result at 64 ticks per second.
const LENGTH_TICKS: u32 = 128;
const RESULT_TICKS: u32 = 64;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(MatchConfig {
            length_seconds: 2.0,
            result_seconds: 1.0,
            ..default()
        })
        .add_plugins(MatchModulePlugin);
    app
}

fn join(app: &mut App, id: u32) -> Entity {
    app.world_mut()
        .spawn((Name::new(player_entity_name(PlayerId(id))), PlayerId(id)))
        .id()
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

fn die(app: &mut App, victim: u32, killer: Option<u32>) {
    app.world_mut().write_message(PlayerDied {
        victim: PlayerId(victim),
        killer: killer.map(PlayerId),
        cause: DamageCause::Hitscan,
    });
}

fn drain<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut()
        .resource_mut::<Messages<M>>()
        .drain()
        .collect()
}

#[test]
fn the_match_starts_with_the_first_player_not_before() {
    let mut app = app();
    ticks(&mut app, 5);
    let error = run(&mut app, "assert.match_running")
        .expect_err("no match")
        .to_string();
    assert!(error.contains("no match yet"), "{error}");
    join(&mut app, 1);
    ticks(&mut app, 1);
    ok(&mut app, "assert.match_running");
    ok(&mut app, "assert.match_score 1 0");
    let world = app.world_mut();
    let mut names = world.query_filtered::<&Name, With<MatchState>>();
    assert_eq!(names.single(world).unwrap().as_str(), MATCH_ENTITY_NAME);
}

#[test]
fn kills_score_and_self_kills_and_world_deaths_cost_a_point() {
    let mut app = app();
    join(&mut app, 1);
    join(&mut app, 2);
    ticks(&mut app, 1);
    die(&mut app, 2, Some(1));
    ticks(&mut app, 1);
    die(&mut app, 2, Some(1));
    die(&mut app, 1, Some(1));
    ticks(&mut app, 1);
    die(&mut app, 2, None);
    // A killer who already left scores nothing; the death still counts.
    die(&mut app, 2, Some(9));
    ticks(&mut app, 1);
    ok(&mut app, "assert.match_score 1 1");
    ok(&mut app, "assert.match_score 2 -1");
    let world = app.world_mut();
    let board = world.query::<&Scoreboard>().single(world).unwrap().clone();
    let lines: Vec<(u32, u32, u32)> = board
        .standings()
        .iter()
        .map(|e| (e.player.0, e.kills, e.deaths))
        .collect();
    assert_eq!(lines, [(1, 2, 1), (2, 0, 4)]);
}

#[test]
fn time_ends_the_match_names_the_winner_and_restarts_it() {
    let mut app = app();
    join(&mut app, 1);
    join(&mut app, 2);
    ticks(&mut app, LENGTH_TICKS - 1);
    ok(&mut app, "assert.match_running");
    // A death in the last tick counts.
    die(&mut app, 1, Some(2));
    ticks(&mut app, 1);
    ok(&mut app, "assert.match_finished");
    ok(&mut app, "assert.match_winner 2");
    assert_eq!(
        drain::<MatchEnded>(&mut app),
        [MatchEnded {
            winner: Some(PlayerId(2))
        }]
    );
    // Deaths after the end do not score.
    die(&mut app, 2, Some(1));
    ticks(&mut app, RESULT_TICKS - 1);
    ok(&mut app, "assert.match_finished");
    ok(&mut app, "assert.match_score 1 0");
    assert!(drain::<MatchRestarted>(&mut app).is_empty());
    ticks(&mut app, 1);
    ok(&mut app, "assert.match_running");
    ok(&mut app, "assert.match_score 2 0");
    assert_eq!(drain::<MatchRestarted>(&mut app).len(), 1);
}

#[test]
fn the_scoreboard_follows_who_is_present() {
    let mut app = app();
    let first = join(&mut app, 1);
    join(&mut app, 2);
    ticks(&mut app, 1);
    die(&mut app, 1, Some(2));
    ticks(&mut app, 1);
    app.world_mut().despawn(first);
    join(&mut app, 3);
    ticks(&mut app, 1);
    ok(&mut app, "assert.match_score 2 1");
    ok(&mut app, "assert.match_score 3 0");
    assert!(run(&mut app, "assert.match_score 1 -1").is_err());
}

#[test]
fn commands_start_end_and_restart() {
    let mut app = app();
    ok(&mut app, "match.start");
    ok(&mut app, "assert.match_running");
    let error = run(&mut app, "match.start")
        .expect_err("running")
        .to_string();
    assert!(error.contains("already running"), "{error}");
    ok(&mut app, "match.end");
    ok(&mut app, "assert.match_winner draw");
    assert!(run(&mut app, "match.end").is_err());
    assert_eq!(drain::<MatchEnded>(&mut app).len(), 1);
    ok(&mut app, "match.start");
    ok(&mut app, "assert.match_running");
    ok(&mut app, "match.restart");
    assert_eq!(drain::<MatchRestarted>(&mut app).len(), 2);
}

#[test]
#[should_panic(expected = "length_seconds")]
fn an_invalid_config_is_refused_at_startup() {
    App::new()
        .insert_resource(MatchConfig {
            length_seconds: 0.0,
            ..default()
        })
        .add_plugins(MatchModulePlugin);
}

fn log(app: &mut App) -> KillLog {
    let mut logs = app.world_mut().query::<&KillLog>();
    logs.single(app.world()).expect("one match").clone()
}

#[test]
fn deaths_are_logged_for_everyone_to_see_and_forgotten_with_the_match() {
    let mut app = app();
    join(&mut app, 1);
    join(&mut app, 2);
    ticks(&mut app, 1);
    die(&mut app, 2, Some(1));
    ticks(&mut app, 1);
    assert_eq!(
        log(&mut app).entries(),
        [KillEntry {
            number: 0,
            victim: PlayerId(2),
            killer: Some(PlayerId(1)),
        }]
    );
    ok(&mut app, "match.restart");
    assert!(log(&mut app).entries().is_empty());
    die(&mut app, 1, None);
    ticks(&mut app, 1);
    assert_eq!(log(&mut app).entries()[0].number, 1);
}

#[test]
fn the_clock_of_the_current_phase_can_be_set() {
    let mut app = app();
    assert!(run(&mut app, "match.set_clock 1").is_err(), "no match");
    join(&mut app, 1);
    ticks(&mut app, 1);
    ok(&mut app, "match.set_clock 0.5");
    ok(&mut app, "assert.match_clock 0.49 0.5");
    ticks(&mut app, 32);
    ok(&mut app, "assert.match_finished");
    ok(&mut app, "match.set_clock 100");
    ticks(&mut app, RESULT_TICKS);
    ok(&mut app, "assert.match_finished");
    assert!(run(&mut app, "match.set_clock 0").is_err());
}
