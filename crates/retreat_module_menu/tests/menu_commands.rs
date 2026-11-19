//! The menu module with only Bevy and the contract crate: the panel it declares, toggling, and the
//! Escape binding. Drawing and clicking are checked by `scripts/verify/module_menu.cmds` in
//! Retreat-Engine-3D.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, InputActions, InputKey, PanelAnchor, PanelDirection, UiPanels,
    run_command_line,
};
use retreat_module_menu::{MENU_ENTRIES, MENU_PANEL_ID, MenuModulePlugin};

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(MenuModulePlugin);
    app
}

#[test]
fn the_panel_is_a_hidden_centered_column_with_a_title() {
    let app = module_app();
    let panels = app.world().resource::<UiPanels>();
    let panel = panels.panel(MENU_PANEL_ID).expect("the module declares it");
    assert_eq!(panel.anchor, PanelAnchor::Center);
    assert_eq!(panel.direction, PanelDirection::Column);
    assert_eq!(panel.title, Some("Paused"));
    assert!(!panels.is_visible(MENU_PANEL_ID), "hidden at the start");
    assert_eq!(panel.entries, MENU_ENTRIES);
}

#[test]
fn toggle_shows_then_hides_the_panel() {
    let mut app = module_app();
    run_command_line(app.world_mut(), "menu.toggle").unwrap();
    assert!(app.world().resource::<UiPanels>().is_visible(MENU_PANEL_ID));
    run_command_line(app.world_mut(), "menu.toggle").unwrap();
    assert!(!app.world().resource::<UiPanels>().is_visible(MENU_PANEL_ID));
}

#[test]
fn escape_runs_the_toggle_command() {
    let app = module_app();
    let actions = app.world().resource::<InputActions>();
    assert_eq!(actions.keys("menu_toggle"), vec![InputKey::Escape]);
    assert_eq!(actions.command_line("menu_toggle"), Some("menu.toggle"));
}

#[test]
fn entries_have_unique_ids_and_the_resume_button_closes_the_menu() {
    let ids: Vec<&str> = MENU_ENTRIES.iter().map(|entry| entry.id).collect();
    assert_eq!(ids, ["resume", "reset", "quit"]);
    assert_eq!(MENU_ENTRIES[0].command_line, "menu.toggle");
}

#[test]
fn toggle_with_an_argument_shows_the_usage() {
    let mut app = module_app();
    let error = run_command_line(app.world_mut(), "menu.toggle now").unwrap_err();
    assert!(error.to_string().contains("usage: menu.toggle"), "{error}");
}
