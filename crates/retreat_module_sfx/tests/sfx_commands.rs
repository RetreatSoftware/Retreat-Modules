//! The sfx module with only Bevy and the contract crate: what it asks the engine to play, naming,
//! and errors. Real playback is checked by `scripts/verify/module_sfx.cmds` in Retreat-Engine-3D.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, InputActions, InputKey, SoundKind, SoundPlayback, run_command_line,
};
use retreat_module_sfx::{MUSIC_ENTITY_NAME, SfxModulePlugin};

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(SfxModulePlugin);
    app
}

/// `(name, sound)` of every sound, sorted by name.
fn sounds(app: &mut App) -> Vec<(String, SoundPlayback)> {
    let world = app.world_mut();
    let mut query = world.query::<(&Name, &SoundPlayback)>();
    let mut all: Vec<(String, SoundPlayback)> = query
        .iter(world)
        .map(|(name, sound)| (name.to_string(), sound.clone()))
        .collect();
    all.sort_by(|(left, _), (right, _)| left.cmp(right));
    all
}

#[test]
fn play_starts_numbered_one_shot_effects() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "sfx.play sounds/a.wav").unwrap();
    run_command_line(app.world_mut(), "sfx.play sounds/b.wav").unwrap();
    let all = sounds(&mut app);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].0, "sfx_1");
    assert_eq!(all[0].1, SoundPlayback::effect("sounds/a.wav"));
    assert_eq!(all[1].0, "sfx_2");
    assert_eq!(all[1].1.kind, SoundKind::Effect);
    assert!(!all[1].1.looping);
}

#[test]
fn a_taken_name_is_skipped() {
    let mut app = module_app();
    app.world_mut().spawn(Name::new("sfx_1"));
    run_command_line(app.world_mut(), "sfx.play sounds/a.wav").unwrap();
    assert_eq!(sounds(&mut app)[0].0, "sfx_2");
}

#[test]
fn music_replaces_music_and_off_stops_it() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "sfx.music sounds/one.wav").unwrap();
    run_command_line(app.world_mut(), "sfx.music sounds/two.wav").unwrap();
    let all = sounds(&mut app);
    assert_eq!(all.len(), 1, "only one music entity at a time");
    assert_eq!(all[0].0, MUSIC_ENTITY_NAME);
    assert_eq!(all[0].1, SoundPlayback::music("sounds/two.wav"));
    run_command_line(app.world_mut(), "sfx.music off").unwrap();
    assert!(sounds(&mut app).is_empty());
}

#[test]
fn the_beep_action_is_bound_to_p_and_runs_play() {
    let app = module_app();
    let actions = app.world().resource::<InputActions>();
    assert_eq!(actions.keys("sfx_beep"), vec![InputKey::P]);
    assert_eq!(
        actions.command_line("sfx_beep"),
        Some("sfx.play sounds/beep.wav")
    );
}

#[test]
fn errors_name_the_command_and_the_usage() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "sfx.play").unwrap_err();
    assert!(
        error.to_string().contains("usage: sfx.play <path>"),
        "{error}"
    );
    let error = run_command_line(app.world_mut(), "sfx.play   ").unwrap_err();
    assert!(
        error.to_string().contains("usage: sfx.play <path>"),
        "{error}"
    );
}
