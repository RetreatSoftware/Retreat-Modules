//! Menu module: decides *what the pause menu offers*; the engine draws it and runs its buttons.
//!
//! The module only declares a `UiPanel` (a centered column with a title, hidden at the start) and one
//! command. It has no UI code, no input code and no platform code:
//! - `menu.toggle`: shows the panel when hidden, hides it when shown.
//! - Action `menu_toggle` (key Escape) runs `menu.toggle`.
//! - Buttons: "Resume" (`menu.toggle`), "Reset boxes" (`physics.reset`, an engine command) and
//!   "Quit" (`quit`, an engine built-in). A game replaces the entries by registering its own panel.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult, InputAction,
    InputKey, PanelAnchor, PanelDirection, UiButtonEntry, UiPanel, UiPanels,
};

/// Id of the menu panel in `ui.*` commands.
pub const MENU_PANEL_ID: &str = "pause_menu";

/// The menu's buttons, in display order.
pub const MENU_ENTRIES: &[UiButtonEntry] = &[
    UiButtonEntry {
        id: "resume",
        label: "Resume",
        command_line: "menu.toggle",
    },
    UiButtonEntry {
        id: "reset",
        label: "Reset boxes",
        command_line: "physics.reset",
    },
    UiButtonEntry {
        id: "quit",
        label: "Quit",
        command_line: "quit",
    },
];

/// Declares the menu panel, `menu.toggle` and the Escape action.
pub struct MenuModulePlugin;

impl Plugin for MenuModulePlugin {
    fn build(&self, app: &mut App) {
        app.register_panel(UiPanel {
            id: MENU_PANEL_ID,
            title: Some("Paused"),
            anchor: PanelAnchor::Center,
            direction: PanelDirection::Column,
            entries: MENU_ENTRIES,
            visible: false,
        })
        .register_command("menu.toggle", "menu.toggle", toggle_command)
        .register_action(InputAction {
            id: "menu_toggle",
            command_line: "menu.toggle",
            default_keys: &[InputKey::Escape],
        });
    }
}

/// `menu.toggle`.
fn toggle_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let mut panels = world.resource_mut::<UiPanels>();
    let show = !panels.is_visible(MENU_PANEL_ID);
    panels
        .set_visible(MENU_PANEL_ID, show)
        .map_err(|message| args.failure(message))?;
    Ok(CommandOutcome::Continue)
}
