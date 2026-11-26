//! Who may shoot.

use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{Health, MatchState, PlayerId};

/// Dead players are not there for a shot (the present decides, not the rewound past).
pub fn dead_players(world: &mut World) -> Vec<Entity> {
    let mut players =
        world.query_filtered::<(Entity, &Health), (With<PlayerId>, Without<RetiredEntity>)>();
    players
        .iter(world)
        .filter(|(_, health)| !health.is_alive())
        .map(|(entity, _)| entity)
        .collect()
}

/// Alive, and no finished match forbids fighting.
pub fn may_fight(world: &mut World, character: Entity) -> bool {
    let alive = world.get::<Health>(character).is_none_or(Health::is_alive);
    let mut matches = world.query_filtered::<&MatchState, Without<RetiredEntity>>();
    alive && matches.iter(world).all(MatchState::allows_combat)
}
