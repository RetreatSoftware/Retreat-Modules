//! Retreat gameplay contract: what gameplay modules share.
//!
//! Modules never depend on each other. When one module must react to another (health to a rifle's
//! hit, a match to a death, a HUD to the score), both use a type from this crate. The crate holds
//! data, messages, system order and pure rules about that data only: no systems, commands, tuning
//! numbers, physics library or transport.
//!
//! Shared components are `NetState` (rolled back and replicated by the engine; follow the rules in
//! `retreat_engine_api`'s `net_state.rs`). Messages are not rewound: see [`GameplaySystems`].
//!
//! Every module that uses a type here calls [`add_gameplay_contract`] in its plugin, so it works
//! when the other modules are absent: a message nobody writes is silence, a message nobody reads is
//! dropped.
//!
//! | Type | Written by | Read by |
//! |---|---|---|
//! | [`PlayerId`] (component) | the module that spawns players | everyone |
//! | [`SpawnPoints`] (resource) | the map module | spawning and respawn |
//! | [`SoloPlayer`] (resource, this machine only) | the module that spawns players | input, camera |
//! | [`LookAngles`] (component) | spawning, then movement | camera, weapons, visuals |
//! | [`Health`] (component) | health | HUD, weapons, visuals |
//! | [`DamageRequested`] | weapons, hazards | health |
//! | [`DamageApplied`] | health | HUD, sounds |
//! | [`PlayerDied`] | health | match, sounds, visuals |
//! | [`PlayerRespawned`] | health | movement, weapons, camera, visuals |
//! | [`Scoreboard`], [`MatchState`] (components of the match entity) | match | health, weapons, HUD |
//! | [`MatchEnded`], [`MatchRestarted`] | match | every module with per-match state |

mod damage;
mod health;
mod local_player;
mod look;
mod match_state;
mod player;
mod schedule;
mod score;
mod spawn;
mod spawn_choice;
mod time;

use bevy::prelude::*;
use retreat_engine_api::{AppNetStateExt, InputActionSystems, add_net_contract};

pub use damage::{DamageApplied, DamageCause, DamageRequested, PlayerDied, PlayerRespawned};
pub use health::{DamageOutcome, Health};
pub use local_player::{SoloPlayer, local_player_id};
pub use look::{LookAngles, horizontal_axes, view_direction};
pub use match_state::{MATCH_ENTITY_NAME, MatchEnded, MatchPhase, MatchRestarted, MatchState};
pub use player::{
    CHARACTER_CAPSULE_HALF_HEIGHT, CHARACTER_EYE_HEIGHT, CHARACTER_HEIGHT, CHARACTER_RADIUS,
    MAX_PLAYERS, PlayerId, character_center, character_feet, player_entity_name,
};
pub use schedule::GameplaySystems;
pub use score::{ScoreEntry, Scoreboard};
pub use spawn::{SpawnPoint, SpawnPoints};
pub use spawn_choice::choose_spawn_point;
pub use time::ticks_from_seconds;

/// Registers the shared net state, messages, resources and system order. Safe to call from several
/// plugins.
pub fn add_gameplay_contract(app: &mut App) {
    add_net_contract(app);
    app.register_net_state::<PlayerId>()
        .register_net_state::<Health>()
        .register_net_state::<MatchState>()
        .register_net_state::<Scoreboard>()
        .register_net_state::<LookAngles>()
        .init_resource::<SoloPlayer>()
        .add_message::<DamageRequested>()
        .add_message::<DamageApplied>()
        .add_message::<PlayerDied>()
        .add_message::<PlayerRespawned>()
        .add_message::<MatchEnded>()
        .add_message::<MatchRestarted>()
        .init_resource::<SpawnPoints>()
        .configure_sets(
            FixedUpdate,
            (
                GameplaySystems::Act,
                GameplaySystems::Damage,
                GameplaySystems::Match,
                GameplaySystems::Restart,
                GameplaySystems::Respawned,
            )
                .chain()
                .after(InputActionSystems),
        );
}
