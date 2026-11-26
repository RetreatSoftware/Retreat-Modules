//! `hud.scoreboard` and `assert.hud_*`.

use crate::show::ScoreboardSwitch;
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandError, CommandOutcome, CommandResult, HudState, UiHud,
};

/// `hud.scoreboard <on|off>`: keeps the scoreboard up without a held button.
pub(crate) fn scoreboard_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let word: String = args.parse(0, "on|off")?;
    let on = match word.as_str() {
        "on" => true,
        "off" => false,
        other => return Err(args.failure(format!("expected `on` or `off`, got `{other}`"))),
    };
    world.resource_mut::<ScoreboardSwitch>().0 = on;
    Ok(CommandOutcome::Continue)
}

fn widget<'w>(world: &'w World, args: &CommandArgs) -> Result<&'w HudState, CommandError> {
    let id: String = args.parse(0, "widget")?;
    world
        .resource::<UiHud>()
        .widget(&id)
        .ok_or_else(|| args.failure(format!("there is no HUD widget `{id}`")))
}

pub(crate) fn assert_shown_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let expected: bool = args.parse(1, "shown")?;
    let state = widget(world, args)?;
    if state.visible != expected {
        return Err(args.failure(format!(
            "widget `{}` shown is {}, expected {expected}",
            state.widget.id, state.visible
        )));
    }
    Ok(CommandOutcome::Continue)
}

/// `assert.hud_text <widget> <words...>`: shown with these words, however they are spaced.
pub(crate) fn assert_text_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    let expected = args.rest(1).join(" ");
    let state = widget(world, args)?;
    let shown: Vec<&str> = state.text.split_whitespace().collect();
    if !state.visible || shown.join(" ") != expected {
        return Err(args.failure(format!(
            "widget `{}` is {} with `{}`, expected it shown with `{expected}`",
            state.widget.id,
            if state.visible { "shown" } else { "hidden" },
            state.text
        )));
    }
    Ok(CommandOutcome::Continue)
}
