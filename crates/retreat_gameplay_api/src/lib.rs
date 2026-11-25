//! Retreat gameplay contract: what gameplay modules share.
//!
//! Modules never depend on each other. When one module must react to another (health to a rifle's
//! hit, a match to a death, a HUD to the score), both use a type from this crate. The crate holds
//! data, messages and pure rules about that data only: no systems, commands, tuning numbers,
//! physics library or networking.
//!
//! Every module that uses a type here calls [`add_gameplay_contract`] in its plugin, so it works
//! when the other modules are absent: a message nobody writes is silence, a message nobody reads is
//! dropped.
//!
//! | Type | Written by | Read by |
//! |---|---|---|
//! | [`PlayerId`] (component) | the module that spawns players | everyone |
//! | [`SpawnPoints`] (resource) | the map module | spawning and respawn |
//! | [`Health`] (component) | health | HUD, weapons, visuals |
//! | [`DamageRequested`] | weapons, hazards | health |
//! | [`DamageApplied`] | health | HUD, sounds |
//! | [`PlayerDied`] | health | match, sounds, visuals |
//! | [`PlayerRespawned`] | health | movement, weapons, camera, visuals |
//! | [`Scoreboard`], [`MatchState`] (resources) | match | HUD |
//! | [`MatchEnded`], [`MatchRestarted`] | match | every module with per-match state |

mod damage;
mod health;
mod match_state;
mod player;
mod score;
mod spawn;

use bevy::prelude::*;

pub use damage::{DamageApplied, DamageCause, DamageRequested, PlayerDied, PlayerRespawned};
pub use health::{DamageOutcome, Health};
pub use match_state::{MatchEnded, MatchPhase, MatchRestarted, MatchState};
pub use player::{
    CHARACTER_EYE_HEIGHT, CHARACTER_HEIGHT, CHARACTER_RADIUS, MAX_PLAYERS, PlayerId,
    player_entity_name,
};
pub use score::{ScoreEntry, Scoreboard};
pub use spawn::{SpawnPoint, SpawnPoints};

/// Registers every shared message and resource. Safe to call from several plugins.
pub fn add_gameplay_contract(app: &mut App) {
    app.add_message::<DamageRequested>()
        .add_message::<DamageApplied>()
        .add_message::<PlayerDied>()
        .add_message::<PlayerRespawned>()
        .add_message::<MatchEnded>()
        .add_message::<MatchRestarted>()
        .init_resource::<SpawnPoints>()
        .init_resource::<Scoreboard>()
        .init_resource::<MatchState>();
}
