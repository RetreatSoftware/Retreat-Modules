//! The shared rules of the gameplay contract.

use bevy::prelude::*;
use retreat_gameplay_api::*;

#[test]
fn damage_removes_hit_points_and_kills_exactly_once() {
    let mut health = Health::full(100);
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 60,
            killed: false
        }
    );
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 40,
            killed: true
        }
    );
    assert!(!health.is_alive());
    assert_eq!(
        health.apply_damage(60),
        DamageOutcome {
            applied: 0,
            killed: false
        }
    );
    health.restore();
    assert_eq!((health.current(), health.max()), (100, 100));
}

#[test]
fn zero_damage_does_nothing_and_nobody_is_born_dead() {
    let mut health = Health::full(0);
    assert!(health.is_alive());
    assert!(!health.apply_damage(0).killed);
}

fn board(lines: &[(u32, i32, u32)]) -> Scoreboard {
    let mut board = Scoreboard::default();
    for &(id, points, deaths) in lines {
        let entry = board.entry_mut(PlayerId(id));
        entry.points = points;
        entry.deaths = deaths;
    }
    board
}

#[test]
fn standings_rank_by_points_then_deaths_then_id() {
    let board = board(&[(3, 2, 0), (1, 5, 4), (2, 5, 1), (0, 2, 0)]);
    let order: Vec<u32> = board.standings().iter().map(|e| e.player.0).collect();
    assert_eq!(order, [2, 1, 0, 3]);
}

#[test]
fn the_winner_needs_strictly_the_most_points() {
    assert_eq!(Scoreboard::default().winner(), None);
    assert_eq!(board(&[(4, -1, 1)]).winner(), Some(PlayerId(4)));
    assert_eq!(board(&[(1, 3, 0), (2, 4, 9)]).winner(), Some(PlayerId(2)));
    assert_eq!(board(&[(1, 3, 0), (2, 3, 2)]).winner(), None);
}

#[test]
fn entries_are_unique_and_removable() {
    let mut board = board(&[(1, 1, 0)]);
    board.entry_mut(PlayerId(1)).kills += 1;
    assert_eq!(board.standings().len(), 1);
    assert_eq!(board.entry(PlayerId(1)).map(|e| e.kills), Some(1));
    board.remove(PlayerId(1));
    assert_eq!(board.entry(PlayerId(1)), None);
}

#[test]
fn only_a_finished_match_stops_combat() {
    let running = MatchState {
        phase: MatchPhase::Running { ticks_left: 1 },
    };
    assert!(running.allows_combat());
    let finished = MatchState {
        phase: MatchPhase::Finished {
            winner: None,
            ticks_until_restart: 1,
        },
    };
    assert!(!finished.allows_combat());
}

#[test]
fn seconds_become_whole_ticks_rounded_up() {
    let tick = std::time::Duration::from_secs_f64(1.0 / 64.0);
    assert_eq!(ticks_from_seconds(3.0, tick), 192);
    assert_eq!(ticks_from_seconds(0.15, tick), 10);
    assert_eq!(ticks_from_seconds(0.001, tick), 1);
    for nothing in [0.0, -1.0, f32::NAN] {
        assert_eq!(ticks_from_seconds(nothing, tick), 0);
    }
}

#[test]
fn the_contract_registers_the_shared_net_state() {
    let mut app = App::new();
    add_gameplay_contract(&mut app);
    let registry = app
        .world()
        .resource::<retreat_engine_api::NetStateRegistry>();
    let ids: Vec<&str> = registry.types().map(|kind| kind.id).collect();
    assert_eq!(
        ids,
        [
            // The contract's networking types bring the engine's own shared state.
            "engine.intangible",
            "engine.pose",
            "gameplay.health",
            "gameplay.inventory",
            "gameplay.kill_log",
            "gameplay.last_pickup",
            "gameplay.look",
            "gameplay.match",
            "gameplay.player",
            "gameplay.respawn",
            "gameplay.scoreboard",
            "gameplay.weapons"
        ]
    );
}

#[test]
fn players_are_named_from_their_id() {
    assert_eq!(player_entity_name(PlayerId(7)), "player_7");
}

#[test]
fn adding_the_contract_twice_registers_everything_once() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    add_gameplay_contract(&mut app);
    app.world_mut()
        .resource_mut::<SpawnPoints>()
        .0
        .push(SpawnPoint {
            feet: Vec3::ZERO,
            yaw: 0.0,
        });
    add_gameplay_contract(&mut app);
    assert_eq!(app.world().resource::<SpawnPoints>().0.len(), 1);
    app.world_mut().write_message(MatchRestarted);
    app.update();
}

#[test]
fn the_kill_log_keeps_the_latest_deaths_and_numbers_them_on() {
    let mut log = KillLog::default();
    for victim in 0..=MAX_PLAYERS as u32 {
        log.push(PlayerId(victim), None);
    }
    assert_eq!(log.entries().len(), MAX_PLAYERS);
    assert_eq!(log.entries()[0].number, 1);
    log.clear();
    log.push(PlayerId(1), Some(PlayerId(2)));
    assert_eq!(log.entries()[0].number, MAX_PLAYERS as u32 + 1);
    assert_eq!(log.next_number(), MAX_PLAYERS as u32 + 2);
}

#[test]
fn players_take_their_name_and_color_from_the_roster_in_turn() {
    assert_eq!(PlayerRoster::default().0.len(), MAX_PLAYERS);
    let roster = PlayerRoster(vec![
        RosterEntry::new("Ada", Color::BLACK),
        RosterEntry::new("Bo", Color::WHITE),
    ]);
    assert_eq!(roster.name(PlayerId(3)), "Bo");
    assert_eq!(roster.color(PlayerId(2)), Color::BLACK);
    assert!(PlayerRoster(Vec::new()).validate().is_err());
}

#[test]
fn a_cue_becomes_a_positional_event_and_others_sound_in_the_past() {
    use retreat_engine_api::{EffectTime, NetLocalClient};
    let cue = SoundCue::new("sounds/a.wav", 0.5);
    let event = cue.event("a/1".to_string(), Some(Vec3::X), EffectTime::Present);
    assert_eq!((event.volume, event.at.unwrap().position), (0.5, Vec3::X));
    assert!(
        cue.event("a/2".to_string(), None, EffectTime::Present)
            .at
            .is_none()
    );
    let client = NetLocalClient(Some(1));
    assert_eq!(event_time(client, PlayerId(1)), EffectTime::Present);
    assert_eq!(event_time(client, PlayerId(2)), EffectTime::RemoteView);
    assert_eq!(
        event_time(NetLocalClient(None), PlayerId(2)),
        EffectTime::Present
    );
    assert!(validate_cues([&None, &Some(cue)]).is_ok());
    assert!(validate_cues([&Some(SoundCue::new("a.wav", 3.0))]).is_err());
}

fn kit() -> ItemDefinition {
    ItemDefinition::new("health_kit", "Health kit", ItemKind::Heal(50), 3)
}

fn rounds() -> ItemDefinition {
    ItemDefinition::new("rounds", "Rounds", ItemKind::Ammo(WeaponKind::Rifle), 90)
}

#[test]
fn an_inventory_holds_one_stack_per_item_up_to_its_limits() {
    let mut inventory = Inventory::new(1);
    assert_eq!(inventory.add(&kit(), 2), 2);
    assert_eq!(inventory.add(&kit(), 5), 1);
    assert_eq!(inventory.room_for(&kit()), 0);
    assert_eq!(inventory.room_for(&rounds()), 0, "no free slot");
    assert_eq!(inventory.take(&kit().id, 5), 3);
    assert!(inventory.stacks().is_empty());
    assert_eq!(inventory.add(&rounds(), 100), 90);
    assert_eq!(inventory.clear().len(), 1);
}

#[test]
fn without_an_inventory_or_an_item_ammunition_is_unlimited_and_weapons_are_carried() {
    let rifle = WeaponKind::Rifle;
    let catalog = ItemCatalog(vec![
        rounds(),
        ItemDefinition::new(
            "launcher",
            "Rocket launcher",
            ItemKind::Weapon(WeaponKind::Launcher),
            1,
        ),
    ]);
    assert_eq!(reserve_of(None, &catalog, rifle), None);
    assert!(owns_weapon(None, &catalog, WeaponKind::Launcher));
    let mut inventory = Inventory::new(4);
    assert!(!owns_weapon(
        Some(&inventory),
        &catalog,
        WeaponKind::Launcher
    ));
    assert!(
        owns_weapon(Some(&inventory), &catalog, rifle),
        "no such item"
    );
    assert_eq!(reserve_of(Some(&inventory), &catalog, rifle), Some(0));
    assert_eq!(
        reserve_of(Some(&inventory), &catalog, WeaponKind::Launcher),
        None
    );
    inventory.add(&rounds(), 40);
    let mut source = AmmoSource {
        inventory: Some(&mut inventory),
        catalog: &catalog,
        weapon: rifle,
    };
    assert_eq!(
        (source.take(30), source.take(30), source.reserve()),
        (30, 10, Some(0))
    );
    let mut unlimited = AmmoSource {
        inventory: None,
        catalog: &catalog,
        weapon: rifle,
    };
    assert_eq!(unlimited.take(30), 30);
}

#[test]
fn a_catalog_names_each_thing_once() {
    assert!(ItemCatalog(vec![kit(), rounds()]).validate().is_ok());
    assert!(ItemCatalog(vec![kit(), kit()]).validate().is_err());
    let more = ItemDefinition::new("more", "More", ItemKind::Ammo(WeaponKind::Rifle), 1);
    assert!(ItemCatalog(vec![rounds(), more]).validate().is_err());
    let nothing = ItemDefinition::new("x", "X", ItemKind::Heal(1), 0);
    assert!(ItemCatalog(vec![nothing]).validate().is_err());
}

#[test]
fn healing_stops_at_full_and_does_not_revive() {
    let mut health = Health::full(100);
    health.apply_damage(70);
    assert_eq!((health.heal(50), health.heal(50)), (50, 20));
    health.apply_damage(100);
    assert_eq!(health.heal(50), 0);
}
