//! How pickups look, on every machine. Render only: derived from the synced state.

use crate::config::LootConfig;
use crate::state::{Pickup, PickupOrigin};
use bevy::prelude::*;
use retreat_engine_api::{HideVisuals, RetiredEntity, SphereVisual};

const GLOW: f32 = 1.2;

type Drawn<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Pickup,
        Option<&'static SphereVisual>,
        Has<HideVisuals>,
    ),
    Without<RetiredEntity>,
>;

pub(crate) fn dress_pickups(pickups: Drawn, config: Res<LootConfig>, mut commands: Commands) {
    for (entity, pickup, sphere, hidden) in &pickups {
        let look = match &pickup.origin {
            PickupOrigin::Spot { kind, .. } => config.kind(kind).map(|kind| kind.look),
            PickupOrigin::Drop { .. } => Some(config.drop_look),
        };
        let Some(look) = look else {
            continue;
        };
        let wanted = SphereVisual {
            radius: look.radius,
            color: look.color,
            glow: GLOW,
        };
        if sphere != Some(&wanted) {
            commands.entity(entity).insert(wanted);
        }
        if pickup.is_there() && hidden {
            commands.entity(entity).remove::<HideVisuals>();
        } else if !pickup.is_there() && !hidden {
            commands.entity(entity).insert(HideVisuals);
        }
    }
}
