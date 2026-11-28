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
//! | [`LastPickup`] (component) | loot | HUD |
//! | [`GrantRequested`] | loot, commands | inventory |
//! | [`ItemsLost`] | inventory | loot |
//! | [`HealRequested`] | inventory | health |
//! | [`MatchEnded`], [`MatchRestarted`] | match | every module with per-match state |
//! | [`PlayingCharacter`] (component, on a body) | spawning, from [`CharacterEntered`] | HUD, every module that saves per character |
//! | [`CharacterEntered`], [`CharacterLeft`] | account | spawning |
//! | [`MapAreas`] (resource) | the map module | areas, spawning |
//! | [`InArea`] (component) | areas | HUD, later PvP rules and portals |
//! | [`GridItemCatalog`] (resource) | the game | every item module, the container UI |
//! | [`GridItem`], [`ItemPlace`] (components of `item_<uid>`, private) | the module whose container holds it; [`move_item`] / [`apply_item_transaction`] | container UI, equipment |
//! | [`ItemContainer`] (component, private) | `grid_inventory` (bag), `equipment`, `town_chest` | container UI, every item move |
//! | [`CharacterLevel`] (component, on a body) | progression (phase 3); `equipment`'s test level until then | equipment, container UI |
//! | [`ItemNotice`] (component, on a body) | item modules (refusals) | container UI |
//! | [`Wallet`] (component of `wallet_<character>`, private) | wallet | container UI |
//! | [`ServiceSpots`] (resource) | the map module | town chest, wallet |

mod account;
mod acting;
mod area;
mod container;
mod damage;
mod grid_item;
mod health;
mod item;
mod item_world;
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
mod wallet;
mod weapon;

use bevy::prelude::*;
use retreat_engine_api::{AppNetStateExt, InputActionSystems, SoundEvents, add_net_contract};

pub use account::{
    AccountId, CharacterEntered, CharacterId, CharacterLeft, PlayingCharacter,
    account_record_owner, character_record_owner,
};
pub use acting::{Route, actor, character_of, issue, refused, route, route_request};
pub use area::{Area, AreaId, AreaRules, InArea, MapAreas};
pub use container::{
    ContainerId, ContainerKind, ContainerLayout, ItemContainer, ItemPlace, MoveContext,
    MoveOutcome, NewStack, PlacedItem, check_equip, container_entity_name, container_id,
    first_free_spot, fits_in_grid, items_under, placed_size, plan_move, plan_stack_add,
};
pub use damage::{
    DamageApplied, DamageCause, DamageRequested, PlayerDied, PlayerFired, PlayerRespawned,
};
pub use grid_item::{
    CharacterLevel, EquipSlot, GridItem, GridItemCatalog, ItemAffix, ItemBase, ItemRarity, ItemUid,
    RolledAffix, item_entity_name,
};
pub use health::{DamageOutcome, Health, RespawnTimer};
pub use item::{
    AmmoSource, GrantRequested, HealRequested, Inventory, ItemCatalog, ItemDefinition, ItemId,
    ItemKind, ItemStack, ItemsLost, LastPickup, owns_weapon, reserve_of,
};
pub use item_world::{
    CONTAINER_RECORD_VERSION, ContainerRecordSeen, ExtraRecords, ITEM_UID_BLOCK,
    ITEM_UID_RECORD_OWNER, ITEM_UID_RECORD_PART, ItemNotice, ItemTransaction, NewItems,
    add_to_container, allocate_item_uids, apply_item_transaction, body_of_character,
    character_level, clear_item_notice, close_orphan_containers, container_entity, container_items,
    container_key, container_of, container_record, in_safe_area, is_authority, items_from_record,
    keep_character_containers, move_item, named_entity, open_container, placed_item, playing_body,
    playing_characters, put_request, read_container, refresh_container_from_record,
    remove_container, show_item_notice, spawn_container, spawn_item, sync_item_privacy,
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
pub use wallet::{ServiceSpot, ServiceSpots, Wallet, wallet_entity_name};
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
    if let Some(catalog) = app.world().get_resource::<GridItemCatalog>() {
        let problems = catalog.problems();
        if !problems.is_empty() {
            panic!("invalid GridItemCatalog:\n{}", problems.join("\n"));
        }
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
        .register_net_state::<LastPickup>()
        .register_net_state::<PlayingCharacter>()
        .register_net_state::<InArea>()
        .register_net_state::<GridItem>()
        .register_net_state::<ItemPlace>()
        .register_net_state::<ItemContainer>()
        .register_net_state::<CharacterLevel>()
        .register_net_state::<ItemNotice>()
        .register_net_state::<Wallet>()
        .init_resource::<GridItemCatalog>()
        .init_resource::<ServiceSpots>()
        .add_message::<CharacterEntered>()
        .add_message::<CharacterLeft>()
        .init_resource::<MapAreas>()
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
