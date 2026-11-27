//! A pickup is heard where its taker is, on every machine, from the synced `LastPickup`.

use crate::config::LootConfig;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, RetiredEntity, SoundEvents};
use retreat_gameplay_api::{LastPickup, PlayerId, event_time};

type Takers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        &'static LastPickup,
        &'static Transform,
    ),
    Without<RetiredEntity>,
>;

/// Plays when a player's count of pickups passes the highest this machine saw. The first sight is
/// silent (a player may join in the middle), and a rewound count is not heard again.
pub(crate) fn play_pickup_sounds(
    players: Takers,
    config: Res<LootConfig>,
    client: Res<NetLocalClient>,
    mut heard: Local<HashMap<Entity, u32>>,
    mut sounds: ResMut<SoundEvents>,
) {
    for (entity, player, last, transform) in &players {
        let Some(before) = heard.get(&entity).copied() else {
            heard.insert(entity, last.count);
            continue;
        };
        if last.count <= before {
            continue;
        }
        heard.insert(entity, last.count);
        let cue = match &last.kind {
            Some(kind) => config.kind(kind).and_then(|kind| kind.sound.as_ref()),
            None => config.drop_sound.as_ref(),
        };
        if let Some(cue) = cue {
            let id = format!("loot/{}/{}", player.0, last.count);
            let time = event_time(*client, *player);
            sounds.play(cue.event(id, Some(transform.translation), time));
        }
    }
}
