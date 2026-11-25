//! Putting the view on screen.

use crate::config::CombatHudConfig;
use crate::view::{HudInput, hud_view};
use crate::{AMMO_WIDGET, CROSSHAIR_WIDGET, HEALTH_WIDGET, HIT_WIDGET, RESPAWN_WIDGET};
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, NetReplaying, RetiredEntity, UiHud};
use retreat_gameplay_api::{
    Health, PlayerId, RespawnTimer, SoloPlayer, WeaponStatus, local_player_id, ticks_from_seconds,
};

/// This machine's running hit marker. Not shared state.
#[derive(Resource, Debug, Default)]
pub(crate) struct HitMarker {
    /// The hit count last seen; `None` before the player has a weapon.
    seen: Option<u32>,
    ticks_left: u32,
}

/// Starts the marker when the server counts a new hit. In ticks, so a script sees the same thing.
pub(crate) fn time_hit_marker(
    players: Query<(&PlayerId, &WeaponStatus), Without<RetiredEntity>>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    replaying: Res<NetReplaying>,
    config: Res<CombatHudConfig>,
    time: Res<Time<Fixed>>,
    mut marker: ResMut<HitMarker>,
) {
    if replaying.0 {
        return;
    }
    let local = local_player_id(*client, *solo);
    let hits = players
        .iter()
        .find(|(player, _)| Some(**player) == local)
        .map(|(_, weapon)| weapon.hits_landed);
    marker.ticks_left = marker.ticks_left.saturating_sub(1);
    if let (Some(before), Some(now)) = (marker.seen, hits)
        && now > before
    {
        marker.ticks_left = ticks_from_seconds(config.hit_marker_seconds, time.timestep()).max(1);
    }
    marker.seen = hits;
}

type Players<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        Option<&'static Health>,
        Option<&'static WeaponStatus>,
        Option<&'static RespawnTimer>,
    ),
    Without<RetiredEntity>,
>;

pub(crate) fn show_hud(
    players: Players,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    marker: Res<HitMarker>,
    config: Res<CombatHudConfig>,
    time: Res<Time<Fixed>>,
    mut hud: ResMut<UiHud>,
) {
    let local = local_player_id(*client, *solo);
    let own = players.iter().find(|(player, ..)| Some(**player) == local);
    let view = hud_view(
        own.map_or_else(HudInput::default, |(_, health, weapon, respawn)| HudInput {
            health: health.copied(),
            weapon: weapon.copied(),
            respawn: respawn.copied(),
            hit: marker.ticks_left > 0,
            timestep: time.timestep(),
        }),
    );
    let texts = [
        (AMMO_WIDGET, view.ammo),
        (HIT_WIDGET, view.hit.then(|| config.hit_marker_text.clone())),
        (HEALTH_WIDGET, view.health),
        (RESPAWN_WIDGET, view.respawn),
    ];
    for (id, text) in texts {
        show(&mut hud, id, text.is_some());
        if let Some(text) = text
            && !hud.has_text(id, &text)
        {
            hud.set_text(id, text).expect("registered by this module");
        }
    }
    show(&mut hud, CROSSHAIR_WIDGET, view.crosshair);
}

/// Changes visibility only when it differs, so the HUD is not rebuilt every frame.
fn show(hud: &mut ResMut<UiHud>, id: &str, visible: bool) {
    if hud.widget(id).is_some_and(|state| state.visible != visible) {
        hud.set_visible(id, visible)
            .expect("registered by this module");
    }
}
