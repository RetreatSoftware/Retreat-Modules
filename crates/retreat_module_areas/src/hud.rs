//! This machine's area name at the top left (render only).

use bevy::prelude::*;
use retreat_engine_api::{NetLocalClient, UiHud};
use retreat_gameplay_api::{InArea, MapAreas, PlayerId, SoloPlayer, local_player_id};

/// The text widget with the area name.
pub const AREA_NAME_WIDGET: &str = "area_name";

/// What players read for an area: its name and whether it is safe.
fn area_label(areas: &MapAreas, in_area: &InArea) -> String {
    match areas.get(&in_area.0.0) {
        Some(area) if area.rules.safe => format!("{} (safe)", area.name),
        Some(area) if area.rules.pvp => format!("{} (PvP)", area.name),
        Some(area) => area.name.clone(),
        None => in_area.0.0.clone(),
    }
}

pub(crate) fn show_area_name(
    areas: Res<MapAreas>,
    client: Res<NetLocalClient>,
    solo: Res<SoloPlayer>,
    characters: Query<(&PlayerId, &InArea)>,
    mut hud: ResMut<UiHud>,
) {
    let local = local_player_id(*client, *solo);
    let label = characters
        .iter()
        .find(|(player, _)| Some(**player) == local)
        .map(|(_, area)| area_label(&areas, area))
        .unwrap_or_default();
    let visible = !label.is_empty();
    let unchanged = hud
        .widget(AREA_NAME_WIDGET)
        .is_some_and(|state| state.text == label && state.visible == visible);
    if !unchanged {
        let _ = hud.set_text(AREA_NAME_WIDGET, label);
        let _ = hud.set_visible(AREA_NAME_WIDGET, visible);
    }
}
