//! How rockets look on each machine. Render only; safe to run again on a re-added rocket.

use crate::config::LauncherConfig;
use crate::state::Rocket;
use bevy::prelude::*;
use retreat_engine_api::{
    HideVisuals, NetLocalClient, RemoteInterpolation, RenderSmoothing, SphereVisual,
};
use retreat_gameplay_api::{SoloPlayer, local_player_id};

const ROCKET_COLOR: Color = Color::srgb(1.0, 0.35, 0.05);

/// The own rocket is drawn at the predicted present, others in the server's past (like their
/// shooters).
pub(crate) fn dress_rockets(
    rockets: Query<(Entity, Ref<Rocket>)>,
    config: Res<LauncherConfig>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    mut commands: Commands,
) {
    let local = local_player_id(*client, *solo);
    for (entity, rocket) in &rockets {
        let others = client.0.is_some() && local != Some(rocket.owner);
        if rocket.is_added() {
            commands.entity(entity).insert(SphereVisual {
                radius: config.rocket_radius,
                color: ROCKET_COLOR,
                glow: 2.0,
            });
            if others {
                commands.entity(entity).insert(RemoteInterpolation);
            } else {
                commands.entity(entity).insert(RenderSmoothing::default());
            }
        }
        // The own rocket vanishes into its explosion; the engine hides another's when the view
        // passes its end.
        if rocket.is_changed() && !others {
            if rocket.exploded > 0 {
                commands.entity(entity).insert(HideVisuals);
            } else {
                commands.entity(entity).remove::<HideVisuals>();
            }
        }
    }
}
