//! Putting the view on screen.

use crate::config::CombatHudConfig;
use crate::view::{HudInput, hud_view};
use crate::{CROSSHAIR_WIDGET, HEALTH_WIDGET, HIT_WIDGET, RESPAWN_WIDGET, ammo_widget};
use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, NetReplaying, RetiredEntity, UiHud};
use retreat_gameplay_api::{
    Health, PlayerId, RespawnTimer, SoloPlayer, WeaponKind, Weapons, local_player_id,
    ticks_from_seconds,
};

/// This machine's running hit marker. Not shared state.
#[derive(Resource, Debug, Default)]
pub(crate) struct HitMarker {
    /// The hit count last seen; `None` before the player has a weapon.
    seen: Option<u32>,
    ticks_left: u32,
    /// Started and not yet on screen: a frame can cover more ticks than the marker lasts, and it
    /// must still be seen for one frame.
    unshown: bool,
}

/// Starts the marker when the server counts a new hit. In ticks, so a script sees the same thing.
pub(crate) fn time_hit_marker(
    players: Query<(&PlayerId, &Weapons), Without<RetiredEntity>>,
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
        .map(|(_, weapons)| weapons.iter().map(|weapon| weapon.hits_landed).sum::<u32>());
    marker.ticks_left = marker.ticks_left.saturating_sub(1);
    if let (Some(before), Some(now)) = (marker.seen, hits)
        && now > before
    {
        marker.ticks_left = ticks_from_seconds(config.hit_marker_seconds, time.timestep()).max(1);
        marker.unshown = true;
    }
    marker.seen = hits;
}

type Players<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        Option<&'static Health>,
        Option<&'static Weapons>,
        Option<&'static RespawnTimer>,
    ),
    Without<RetiredEntity>,
>;

pub(crate) fn show_hud(
    players: Players,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    mut marker: ResMut<HitMarker>,
    config: Res<CombatHudConfig>,
    time: Res<Time<Fixed>>,
    mut hud: ResMut<UiHud>,
) {
    let local = local_player_id(*client, *solo);
    let own = players.iter().find(|(player, ..)| Some(**player) == local);
    let view = hud_view(
        own.map_or_else(HudInput::default, |(_, health, weapons, respawn)| {
            HudInput {
                health: health.copied(),
                weapons: weapons.cloned(),
                respawn: respawn.copied(),
                hit: marker.ticks_left > 0 || marker.unshown,
                timestep: time.timestep(),
            }
        }),
    );
    marker.unshown = false;
    let ammo = WeaponKind::ALL.map(|kind| {
        let line = view.ammo.iter().find(|(found, _)| *found == kind);
        (ammo_widget(kind), line.map(|(_, text)| text.clone()))
    });
    let texts = [
        (HIT_WIDGET, view.hit.then(|| config.hit_marker_text.clone())),
        (HEALTH_WIDGET, view.health),
        (RESPAWN_WIDGET, view.respawn),
    ];
    for (id, text) in texts.into_iter().chain(ammo) {
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
