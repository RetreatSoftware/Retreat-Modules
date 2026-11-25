//! `assert.hud_shown`.

use bevy::prelude::*;
use retreat_engine_api::{CommandArgs, CommandOutcome, CommandResult, UiHud};

pub(crate) fn assert_shown_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let widget: String = args.parse(0, "widget")?;
    let expected: bool = args.parse(1, "shown")?;
    let Some(state) = world.resource::<UiHud>().widget(&widget) else {
        return Err(args.failure(format!("there is no HUD widget `{widget}`")));
    };
    if state.visible != expected {
        return Err(args.failure(format!(
            "widget `{widget}` shown is {}, expected {expected}",
            state.visible
        )));
    }
    Ok(CommandOutcome::Continue)
}
