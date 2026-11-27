//! A used item is heard where its user is, on every machine.

use crate::config::InventoryConfig;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, NetState, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{PlayerId, event_time};
use serde::{Deserialize, Serialize};

/// On every player: how many items it used, so each use has its own sound id.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemUses(pub u32);

impl NetState for ItemUses {
    const ID: &'static str = "inventory.uses";
}

type Users<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        &'static ItemUses,
        &'static Transform,
    ),
    Without<RetiredEntity>,
>;

/// Plays when a player's count of uses passes the highest this machine saw. The first sight is
/// silent (a player may join in the middle), and a replayed use is not heard again.
pub(crate) fn play_use_sounds(
    players: Users,
    config: Res<InventoryConfig>,
    client: Res<NetLocalClient>,
    mut heard: Local<HashMap<Entity, u32>>,
    mut sounds: ResMut<SoundEvents>,
) {
    for (entity, player, uses, transform) in &players {
        let Some(before) = heard.get(&entity).copied() else {
            heard.insert(entity, uses.0);
            continue;
        };
        if uses.0 <= before {
            continue;
        }
        heard.insert(entity, uses.0);
        if let Some(cue) = &config.use_sound {
            let id = format!("item/{}/{}", player.0, uses.0);
            let time = event_time(*client, *player);
            sounds.play(cue.event(id, Some(transform.translation), time));
        }
    }
}
