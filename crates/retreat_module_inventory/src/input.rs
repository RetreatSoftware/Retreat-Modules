//! This machine's use button, turned into a command line.

use bevy::prelude::*;
use retreat_engine_api::{InputActions, NetLocalClient, NetReplaying};
use retreat_gameplay_api::{
    Inventory, ItemCatalog, ItemKind, SoloPlayer, character_of, issue, local_player_id,
};

/// A press uses the first usable item carried, in catalog order.
pub(crate) fn send_input(world: &mut World, mut was_down: Local<bool>) {
    // A replay runs the recorded inputs; this tick's buttons are not new.
    if world.resource::<NetReplaying>().0 {
        return;
    }
    let down = world.resource::<InputActions>().is_held("inventory_use");
    let pressed = down && !*was_down;
    *was_down = down;
    if !pressed {
        return;
    }
    let local = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    );
    let Some(character) = local.and_then(|local| character_of(world, local)) else {
        return;
    };
    let Some(inventory) = world.get::<Inventory>(character) else {
        return;
    };
    let usable = world
        .resource::<ItemCatalog>()
        .0
        .iter()
        .find(|item| matches!(item.kind, ItemKind::Heal(_)) && inventory.count(&item.id) > 0)
        .map(|item| item.id.clone());
    // Nothing to heal is not worth a command (and would be refused with a warning without a
    // network).
    let wounded = world
        .get::<retreat_gameplay_api::Health>(character)
        .is_some_and(|health| health.is_alive() && health.current() < health.max());
    if let Some(item) = usable
        && wounded
    {
        issue(world, format!("inventory.use {item}"));
    }
}
