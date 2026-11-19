//! The highscore module with only Bevy and the contract crate: commands, saving and loading
//! through `SaveStore`, migration of older saves, and errors.

use bevy::prelude::*;
use retreat_engine_api::serde_json::json;
use retreat_engine_api::{CommandRegistry, SaveStore, run_command_line};
use retreat_module_highscore::{DATA_VERSION, HighscoreModulePlugin, MODULE_ID, Score};

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(SaveStore::in_memory())
        .add_plugins(HighscoreModulePlugin);
    app
}

fn run(app: &mut App, line: &str) {
    run_command_line(app.world_mut(), line).unwrap_or_else(|error| panic!("{line}: {error}"));
}

fn score(app: &App) -> Score {
    *app.world().resource::<Score>()
}

#[test]
fn add_raises_the_score_and_the_best_and_reset_keeps_the_best() {
    let mut app = module_app();
    run(&mut app, "score.add 30");
    run(&mut app, "score.add 12");
    assert_eq!(
        score(&app),
        Score {
            current: 42,
            best: 42
        }
    );
    run(&mut app, "score.reset");
    run(&mut app, "score.add 5");
    assert_eq!(
        score(&app),
        Score {
            current: 5,
            best: 42
        }
    );
}

#[test]
fn save_then_load_restores_the_score_in_a_new_run() {
    let mut first = module_app();
    run(&mut first, "score.add 77");
    run(&mut first, "score.save");
    let saved = first
        .world()
        .resource::<SaveStore>()
        .load(MODULE_ID)
        .unwrap()
        .unwrap();
    assert_eq!(saved.version, DATA_VERSION);
    assert_eq!(saved.data, json!({"score": 77, "best": 77}));

    // A "new run": a fresh app whose store holds the saved document.
    let mut second = module_app();
    second
        .world_mut()
        .resource_mut::<SaveStore>()
        .save(MODULE_ID, saved.version, &saved.data)
        .unwrap();
    run(&mut second, "score.load");
    assert_eq!(
        score(&second),
        Score {
            current: 77,
            best: 77
        }
    );
}

#[test]
fn loading_with_no_save_keeps_the_score() {
    let mut app = module_app();
    run(&mut app, "score.add 3");
    run(&mut app, "score.load");
    assert_eq!(score(&app).current, 3);
}

#[test]
fn an_older_save_is_migrated_and_a_newer_one_is_refused() {
    let mut app = module_app();
    app.world_mut()
        .resource_mut::<SaveStore>()
        .save(MODULE_ID, 1, &json!({"points": 55}))
        .unwrap();
    run(&mut app, "score.load");
    assert_eq!(
        score(&app),
        Score {
            current: 55,
            best: 55
        }
    );

    app.world_mut()
        .resource_mut::<SaveStore>()
        .save(MODULE_ID, DATA_VERSION + 1, &json!({}))
        .unwrap();
    let error = run_command_line(app.world_mut(), "score.load").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("version 3, but this module reads up to 2"),
        "{error}"
    );
}

#[test]
fn a_damaged_save_is_reported_with_the_fix() {
    let mut app = module_app();
    app.world_mut()
        .resource_mut::<SaveStore>()
        .save(MODULE_ID, DATA_VERSION, &json!({"score": "many"}))
        .unwrap();
    let error = run_command_line(app.world_mut(), "score.load").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("no whole number `score`"), "{message}");
    assert!(message.contains("save.delete highscore"), "{message}");
}

#[test]
fn errors_name_the_command_and_the_limit() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "score.add 2000000").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("<points> must be from 0 to 1000000"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "score.add -1").unwrap_err();
    assert!(
        error.to_string().contains("usage: score.add <points>"),
        "{error}"
    );
}
