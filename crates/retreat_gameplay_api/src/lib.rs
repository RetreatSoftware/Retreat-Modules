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
//! | [`PickupSpots`] (resource) | the map module | loot |
//! | [`SoloPlayer`] (resource, this machine only) | the module that spawns players | input, camera |
//! | [`LookAngles`] (component) | spawning, then movement | camera, weapons, visuals |
//! | [`Health`] (component) | health | HUD, weapons, visuals |
//! | [`RespawnTimer`] (component, on the dead) | health | HUD |
//! | [`Weapons`] (component) | weapons, one entry each | HUD |
//! | [`CharacterSize`] (resource) | the game | spawning, weapons, camera, visuals, respawn |
//! | [`PlayerFired`] | weapons | health |
//! | [`DamageRequested`] | weapons, hazards | health |
//! | [`DamageApplied`] | health | HUD, sounds |
//! | [`PlayerDied`] | health | match, sounds, visuals |
//! | [`PlayerRespawned`] | health | movement, weapons, camera, visuals |
//! | [`Scoreboard`], [`MatchState`] (components of the match entity) | match | health, weapons, HUD |
//! | [`KillLog`] (component of the match entity) | match | HUD |
//! | [`PlayerRoster`] (resource) | the game | visuals, HUD |
//! | [`SoundCue`] (in module configs) | the game | modules that play sounds |
//! | [`ItemCatalog`] (resource) | the game | inventory, weapons, HUD, loot |
//! | [`Inventory`] (component) | inventory; weapons take their ammunition | weapons, HUD, loot |
//! | [`GrantRequested`] | loot, commands | inventory |
//! | [`ItemsLost`] | inventory | loot |
//! | [`HealRequested`] | inventory | health |
//! | [`MatchEnded`], [`MatchRestarted`] | match | every module with per-match state |

mod acting;
mod damage;
mod health;
mod item;
mod kill_log;
mod local_player;
mod look;
mod match_state;
mod player;
mod roster;
mod schedule;
mod score;
mod sound;
mod spawn;
mod spawn_choice;
mod time;
mod weapon;

use bevy::prelude::*;
use retreat_engine_api::{AppNetStateExt, InputActionSystems, SoundEvents, add_net_contract};

pub use acting::{actor, character_of, issue, refused};
pub use damage::{
    DamageApplied, DamageCause, DamageRequested, PlayerDied, PlayerFired, PlayerRespawned,
};
pub use health::{DamageOutcome, Health, RespawnTimer};
pub use item::{
    AmmoSource, GrantRequested, HealRequested, Inventory, ItemCatalog, ItemDefinition, ItemId,
    ItemKind, ItemStack, ItemsLost, owns_weapon, reserve_of,
};
pub use kill_log::{KillEntry, KillLog};
pub use local_player::{SoloPlayer, local_player_id};
pub use look::{LookAngles, horizontal_axes, view_direction};
pub use match_state::{MATCH_ENTITY_NAME, MatchEnded, MatchPhase, MatchRestarted, MatchState};
pub use player::{CharacterSize, MAX_PLAYERS, PlayerId, player_entity_name};
pub use roster::{PlayerRoster, RosterEntry};
pub use schedule::GameplaySystems;
pub use score::{ScoreEntry, Scoreboard};
pub use sound::{SoundCue, event_time, validate_cues};
pub use spawn::{PickupSpot, PickupSpots, SpawnPoint, SpawnPoints};
pub use spawn_choice::choose_spawn_point;
pub use time::ticks_from_seconds;
pub use weapon::{WeaponKind, WeaponStatus, Weapons};

/// Registers the shared net state, messages, resources and system order. Safe to call from several
/// plugins.
pub fn add_gameplay_contract(app: &mut App) {
    add_net_contract(app);
    if let Some(roster) = app.world().get_resource::<PlayerRoster>()
        && let Err(problem) = roster.validate()
    {
        panic!("invalid PlayerRoster: {problem}");
    }
    if let Some(catalog) = app.world().get_resource::<ItemCatalog>()
        && let Err(problem) = catalog.validate()
    {
        panic!("invalid ItemCatalog: {problem}");
    }
    if let Some(size) = app.world().get_resource::<CharacterSize>()
        && let Err(problem) = size.validate()
    {
        panic!("invalid CharacterSize: {problem}");
    }
    app.register_net_state::<PlayerId>()
        .register_net_state::<Health>()
        .register_net_state::<MatchState>()
        .register_net_state::<Scoreboard>()
        .register_net_state::<LookAngles>()
        .register_net_state::<RespawnTimer>()
        .register_net_state::<Weapons>()
        .register_net_state::<KillLog>()
        .register_net_state::<Inventory>()
        .init_resource::<ItemCatalog>()
        .add_message::<GrantRequested>()
        .add_message::<ItemsLost>()
        .add_message::<HealRequested>()
        .init_resource::<PlayerRoster>()
        .init_resource::<SoundEvents>()
        .init_resource::<CharacterSize>()
        .add_message::<PlayerFired>()
        .init_resource::<SoloPlayer>()
        .add_message::<DamageRequested>()
        .add_message::<DamageApplied>()
        .add_message::<PlayerDied>()
        .add_message::<PlayerRespawned>()
        .add_message::<MatchEnded>()
        .add_message::<MatchRestarted>()
        .init_resource::<SpawnPoints>()
        .init_resource::<PickupSpots>()
        .configure_sets(
            FixedUpdate,
            (
                GameplaySystems::Act,
                GameplaySystems::Damage,
                GameplaySystems::Pickup,
                GameplaySystems::Items,
                GameplaySystems::Drops,
                GameplaySystems::Match,
                GameplaySystems::Restart,
                GameplaySystems::Respawned,
            )
                .chain()
                .after(InputActionSystems),
        );
}
