//! Creating, filling and emptying inventories.

use crate::config::InventoryConfig;
use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{
    GrantRequested, Health, Inventory, ItemCatalog, ItemsLost, MatchRestarted, PlayerDied,
    PlayerId, PlayerRespawned,
};

type Empty = (With<PlayerId>, Without<Inventory>, Without<RetiredEntity>);

pub(crate) fn attach_inventories(
    players: Query<Entity, Empty>,
    config: Res<InventoryConfig>,
    catalog: Res<ItemCatalog>,
    mut commands: Commands,
) {
    for entity in &players {
        commands.entity(entity).insert(config.starting(&catalog));
    }
}

/// Grants in the order they were asked for.
pub(crate) fn apply_grants(
    mut grants: MessageReader<GrantRequested>,
    mut players: Query<(&PlayerId, &mut Inventory, Option<&Health>), Without<RetiredEntity>>,
    catalog: Res<ItemCatalog>,
) {
    for grant in grants.read() {
        let Some(item) = catalog.get(&grant.item) else {
            warn!("inventory: there is no item `{}` to grant", grant.item);
            continue;
        };
        for (player, mut inventory, health) in &mut players {
            if *player == grant.player && health.is_none_or(Health::is_alive) {
                let mut next = inventory.clone();
                next.add(item, grant.count);
                inventory.set_if_neq(next);
            }
        }
    }
}

pub(crate) fn lose_on_death(
    mut deaths: MessageReader<PlayerDied>,
    mut players: Query<(&PlayerId, &mut Inventory, Option<&Transform>), Without<RetiredEntity>>,
    mut lost: MessageWriter<ItemsLost>,
) {
    for death in deaths.read() {
        for (player, mut inventory, transform) in &mut players {
            if *player == death.victim && !inventory.stacks().is_empty() {
                lost.write(ItemsLost {
                    player: *player,
                    items: inventory.clear(),
                    at: transform.map_or(Vec3::ZERO, |transform| transform.translation),
                });
            }
        }
    }
}

/// A new match: everyone starts over (also without a health module to respawn anyone).
pub(crate) fn reset_on_restart(
    mut restarts: MessageReader<MatchRestarted>,
    mut players: Query<&mut Inventory, Without<RetiredEntity>>,
    config: Res<InventoryConfig>,
    catalog: Res<ItemCatalog>,
) {
    if restarts.read().count() == 0 {
        return;
    }
    for mut inventory in &mut players {
        inventory.set_if_neq(config.starting(&catalog));
    }
}

pub(crate) fn reset_on_respawn(
    mut respawned: MessageReader<PlayerRespawned>,
    mut players: Query<(&PlayerId, &mut Inventory), Without<RetiredEntity>>,
    config: Res<InventoryConfig>,
    catalog: Res<ItemCatalog>,
) {
    for message in respawned.read() {
        for (player, mut inventory) in &mut players {
            if *player == message.player {
                inventory.set_if_neq(config.starting(&catalog));
            }
        }
    }
}
