//! Areas (crawler plan, phase 1): the loaded map's areas (`MapAreas`, map data: the safe city,
//! dungeons with tier and maximum entry level) and which one each character is in.
//!
//! - **Exactly one area per character** ([`InArea`], shared state): every tick a character in an
//!   area's rectangle is in that area ([`area_for`]); one outside every rectangle (falling, between
//!   areas) keeps the area it had, and a new one outside every area is in the home area; a pending
//!   `PhysicsTeleport` counts as where the character is. The rule runs the same way on the server
//!   and on clients.
//! - `areas.move <player> <area>` puts a character at a spawn point of another area (the one
//!   farthest from others, like spawning). Decided where the game is decided; clients cannot send
//!   it (portals will, in a later phase, through their own rules).
//! - This machine's player sees the name of its area at the top left (`AreasConfig::show_area_name`).
//!
//! The city and the dungeons are areas of one physics world for now; this module is the seam where
//! areas later move to servers of their own.

mod commands;
mod hud;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppHudExt, HudAnchor, HudKind, HudWidget, PhysicsTeleport,
};
use retreat_gameplay_api::{
    Area, AreaId, CharacterSize, GameplaySystems, InArea, MapAreas, PlayerId, add_gameplay_contract,
};

pub use hud::AREA_NAME_WIDGET;

/// Insert before adding the plugin to change it.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreasConfig {
    /// Show this machine's area name at the top left.
    pub show_area_name: bool,
}

impl Default for AreasConfig {
    fn default() -> Self {
        Self {
            show_area_name: true,
        }
    }
}

/// The area a character at `feet` is in, given the area it had: the area whose rectangle holds
/// it, else the one it had, else the home area. Pure.
pub fn area_for<'a>(areas: &'a MapAreas, feet: Vec3, had: Option<&AreaId>) -> Option<&'a Area> {
    areas
        .at(feet)
        .or_else(|| had.and_then(|id| areas.get(&id.0)))
        .or_else(|| areas.home())
}

/// Adds areas and the `areas.*` commands.
pub struct AreasModulePlugin;

impl Plugin for AreasModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<AreasConfig>();
        let config = *app.world().resource::<AreasConfig>();
        app.register_command(
            "areas.move",
            "areas.move <player> <area>",
            commands::move_command,
        )
        .register_command("areas.list", "areas.list", commands::list_command)
        .register_command(
            "assert.area",
            "assert.area <player> <area>",
            commands::assert_area_command,
        )
        .register_hud_widget(HudWidget {
            id: AREA_NAME_WIDGET,
            anchor: HudAnchor::TopLeft,
            kind: HudKind::Text { font_size: 20.0 },
            visible: false,
        })
        .add_systems(
            FixedUpdate,
            // After spawning (Act) so a new character has its area in the tick it appears.
            keep_characters_in_areas.after(GameplaySystems::Act),
        );
        if config.show_area_name {
            app.add_systems(Update, hud::show_area_name);
        }
    }
}

/// What `keep_characters_in_areas` reads and writes of a character.
type CharacterArea = (
    Entity,
    &'static Transform,
    Option<&'static PhysicsTeleport>,
    Option<&'static mut InArea>,
);

/// Gives every character the area it is in (only when that changes).
fn keep_characters_in_areas(
    areas: Res<MapAreas>,
    size: Res<CharacterSize>,
    mut commands: Commands,
    mut characters: Query<CharacterArea, With<PlayerId>>,
) {
    if areas.0.is_empty() {
        return;
    }
    for (entity, transform, teleport, current) in &mut characters {
        // A teleport not applied yet (it is applied in the physics step) already decides the area.
        let center = teleport.map_or(transform.translation, |teleport| teleport.0);
        let feet = size.feet(center);
        let had = current.as_ref().map(|area| &area.0);
        let Some(area) = area_for(&areas, feet, had) else {
            continue;
        };
        match current {
            Some(mut current) if current.0 != area.id => current.0 = area.id.clone(),
            Some(_) => {}
            None => {
                commands.entity(entity).insert(InArea(area.id.clone()));
            }
        }
    }
}
