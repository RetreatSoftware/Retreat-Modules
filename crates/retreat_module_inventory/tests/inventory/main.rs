//! The inventory alone: only Bevy and the contract crates. With weapons, health and a network it is
//! checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_inventory::{InventoryConfig, InventoryModulePlugin};

fn catalog() -> ItemCatalog {
    ItemCatalog(vec![
        ItemDefinition::new("rounds", "Rounds", ItemKind::Ammo(WeaponKind::Rifle), 90),
        ItemDefinition::new("kit", "Health kit", ItemKind::Heal(50), 3),
    ])
}

/// Player 1, this machine's, with the loadout of one kit and 30 rounds.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<InputActions>()
        .insert_resource(catalog())
        .insert_resource(InventoryConfig {
            slots: 2,
            loadout: vec![(ItemId::new("rounds"), 30), (ItemId::new("kit"), 1)],
        })
        .add_plugins(InventoryModulePlugin);
    app.world_mut().spawn((
        Name::new(player_entity_name(PlayerId(1))),
        PlayerId(1),
        Health::full(100),
        Transform::from_xyz(2.0, 1.0, 3.0),
    ));
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(1))));
    ticks(&mut app, 1);
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn ticks(app: &mut App, count: u32) {
    for _ in 0..count {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn wound(app: &mut App, amount: u32) {
    let mut players = app.world_mut().query::<&mut Health>();
    players
        .single_mut(app.world_mut())
        .expect("one player")
        .apply_damage(amount);
}

fn heals(app: &mut App) -> Vec<HealRequested> {
    app.world_mut()
        .resource_mut::<Messages<HealRequested>>()
        .drain()
        .collect()
}

#[test]
fn a_player_starts_with_the_loadout_and_grants_stop_at_the_limit() {
    let mut app = app();
    ok(&mut app, "assert.inventory 1 rounds 30");
    ok(&mut app, "assert.inventory 1 kit 1");
    ok(&mut app, "inventory.grant 1 kit 9");
    ticks(&mut app, 1);
    ok(&mut app, "assert.inventory 1 kit 3");
    assert!(run(&mut app, "inventory.grant 1 gold 1").is_err());
    assert!(run(&mut app, "inventory.grant 7 kit 1").is_err());
}

#[test]
fn a_health_kit_is_used_only_when_it_helps() {
    let mut app = app();
    assert!(run(&mut app, "inventory.use kit").is_err(), "full health");
    assert!(run(&mut app, "inventory.use rounds").is_err());
    wound(&mut app, 60);
    ok(&mut app, "inventory.use kit");
    assert_eq!(
        heals(&mut app),
        [HealRequested {
            player: PlayerId(1),
            amount: 50,
        }]
    );
    ok(&mut app, "assert.inventory 1 kit 0");
    assert!(run(&mut app, "inventory.use kit").is_err(), "none left");
    // A network input that came too late is silently ignored.
    app.world_mut().insert_resource(NetCommandSender(Some(1)));
    ok(&mut app, "inventory.use kit");
}

#[test]
fn the_use_key_uses_a_kit_once_per_press() {
    let mut app = app();
    ok(&mut app, "inventory.grant 1 kit 2");
    wound(&mut app, 90);
    app.world_mut()
        .resource_mut::<InputActions>()
        .update_held(|key| key == InputKey::E);
    ticks(&mut app, 10);
    ok(&mut app, "assert.inventory 1 kit 2");
}

#[test]
fn a_finished_match_stops_the_use_of_items() {
    let mut app = app();
    wound(&mut app, 60);
    app.world_mut().spawn(MatchState {
        phase: MatchPhase::Finished {
            winner: None,
            ticks_until_restart: 10,
        },
    });
    assert!(run(&mut app, "inventory.use kit").is_err());
}

#[test]
fn death_empties_it_and_says_what_was_lost_and_a_new_life_starts_over() {
    let mut app = app();
    app.world_mut().write_message(PlayerDied {
        victim: PlayerId(1),
        killer: None,
        cause: DamageCause::World,
    });
    ticks(&mut app, 1);
    ok(&mut app, "assert.inventory 1 rounds 0");
    let lost: Vec<ItemsLost> = app
        .world_mut()
        .resource_mut::<Messages<ItemsLost>>()
        .drain()
        .collect();
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0].at, Vec3::new(2.0, 1.0, 3.0));
    assert_eq!(lost[0].items.len(), 2);
    app.world_mut().write_message(PlayerRespawned {
        player: PlayerId(1),
        spawn: SpawnPoint {
            feet: Vec3::ZERO,
            yaw: 0.0,
        },
    });
    ticks(&mut app, 1);
    ok(&mut app, "assert.inventory 1 rounds 30");
}

#[test]
fn a_new_match_resets_every_inventory() {
    let mut app = app();
    ok(&mut app, "inventory.grant 1 rounds 40");
    ticks(&mut app, 1);
    app.world_mut().write_message(MatchRestarted);
    ticks(&mut app, 1);
    ok(&mut app, "assert.inventory 1 rounds 30");
}

#[test]
#[should_panic(expected = "does not fit")]
fn a_loadout_that_does_not_fit_is_refused_at_startup() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(catalog())
        .insert_resource(InventoryConfig {
            slots: 2,
            loadout: vec![(ItemId::new("kit"), 9)],
        })
        .add_plugins(InventoryModulePlugin);
}
