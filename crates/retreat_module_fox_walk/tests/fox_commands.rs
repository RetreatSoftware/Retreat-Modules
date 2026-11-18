//! `fox.set_speed` with only Bevy and the contract crate: what it writes on the model, and errors.
//! The rendered result is checked by `scripts/verify/module_fox_walk.cmds` in Retreat-Engine-3D.

use bevy::prelude::*;
use retreat_engine_api::{AnimationPlayback, CommandRegistry, run_command_line};
use retreat_module_fox_walk::FoxWalkModulePlugin;

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(FoxWalkModulePlugin);
    app.world_mut().spawn(Name::new("fox"));
    app
}

fn playback(app: &mut App) -> AnimationPlayback {
    let world = app.world_mut();
    let mut query = world.query::<&AnimationPlayback>();
    query
        .single(world)
        .expect("one model with a playback")
        .clone()
}

#[test]
fn set_speed_writes_the_clips_for_that_speed() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "fox.set_speed fox 2").unwrap();
    let clips: Vec<(String, f32)> = playback(&mut app)
        .clips
        .into_iter()
        .map(|clip| (clip.clip, clip.weight))
        .collect();
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].0, "Walk");
    assert_eq!(clips[1].0, "Run");
}

#[test]
fn a_later_speed_replaces_the_list() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "fox.set_speed fox 2").unwrap();
    run_command_line(app.world_mut(), "fox.set_speed fox 0").unwrap();
    let clips = playback(&mut app).clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].clip, "Survey");
}

#[test]
fn errors_name_the_command_the_value_and_the_fix() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "fox.set_speed fox 99").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("<speed> must be from 0 to 10, got 99"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "fox.set_speed ghost 1").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("expected exactly one entity named `ghost`, found 0"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "fox.set_speed fox").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("usage: fox.set_speed <model> <speed>"),
        "{error}"
    );
}
