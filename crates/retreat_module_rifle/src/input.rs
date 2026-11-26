//! This machine's rifle buttons, turned into command lines.

use crate::config::RifleConfig;
use crate::state::RifleState;
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
        kind: WeaponKind::Rifle,
        fire_action: "rifle_fire",
        reload_action: "rifle_reload",
        range: world.resource::<RifleConfig>().range,
    };
    let cooldown = |world: &World, character: Entity| {
        world
            .get::<RifleState>(character)
            .map(|rifle| rifle.cooldown_ticks)
    };
    match pull_trigger(world, trigger, cooldown, &mut memory) {
        Some(TriggerOrder::Reload) => issue(world, "rifle.reload".to_string()),
        Some(TriggerOrder::Fire { aim, view_tick }) => issue(
            world,
            format!("rifle.fire {} {} {view_tick}", aim.yaw, aim.pitch),
        ),
        None => {}
    }
}
