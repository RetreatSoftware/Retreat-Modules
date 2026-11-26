//! This machine's launcher buttons, turned into command lines.

use crate::config::LauncherConfig;
use crate::state::LauncherState;
use bevy::prelude::*;
use retreat_engine_api::NetReplaying;
use retreat_gameplay_api::WeaponKind;
use retreat_weapon_kit::{Trigger, TriggerMemory, TriggerOrder, issue, pull_trigger};

pub(crate) fn send_input(world: &mut World, mut memory: Local<TriggerMemory>) {
    // A replay runs the recorded inputs; this tick's buttons are not new.
    if world.resource::<NetReplaying>().0 {
        return;
    }
    let trigger = Trigger {
        kind: WeaponKind::Launcher,
        fire_action: "launcher_fire",
        reload_action: "launcher_reload",
        range: world.resource::<LauncherConfig>().aim_range,
    };
    let cooldown = |world: &World, character: Entity| {
        world
            .get::<LauncherState>(character)
            .map(|launcher| launcher.cooldown_ticks)
    };
    match pull_trigger(world, trigger, cooldown, &mut memory) {
        Some(TriggerOrder::Reload) => issue(world, "launcher.reload".to_string()),
        Some(TriggerOrder::Fire { aim, .. }) => {
            issue(world, format!("launcher.fire {} {}", aim.yaw, aim.pitch));
        }
        None => {}
    }
}
