//! Who may shoot, and what a shot passes through.

use bevy::prelude::*;
use retreat_engine_api::RetiredEntity;
use retreat_gameplay_api::{Health, MatchState, PlayerId};

/// The character of `player`, if there is one.
pub(crate) fn character_of(world: &mut World, player: PlayerId) -> Option<Entity> {
    let mut players = world.query_filtered::<(Entity, &PlayerId), Without<RetiredEntity>>();
    players
        .iter(world)
        .find(|(_, found)| **found == player)
        .map(|(entity, _)| entity)
}

/// Dead players are not there for a shot (the present decides, not the rewound past).
pub(crate) fn dead_players(world: &mut World) -> Vec<Entity> {
    let mut players =
        world.query_filtered::<(Entity, &Health), (With<PlayerId>, Without<RetiredEntity>)>();
    players
        .iter(world)
        .filter(|(_, health)| !health.is_alive())
        .map(|(entity, _)| entity)
        .collect()
}

/// Alive, and no finished match forbids fighting.
pub(crate) fn may_fight(world: &mut World, character: Entity) -> bool {
    let alive = world.get::<Health>(character).is_none_or(Health::is_alive);
    let mut matches = world.query_filtered::<&MatchState, Without<RetiredEntity>>();
    alive && matches.iter(world).all(MatchState::allows_combat)
}
