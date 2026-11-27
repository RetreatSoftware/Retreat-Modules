//! The loot alone: only Bevy and the contract crates. Grants are read as the inventory would; with
//! the real inventory, weapons and a network it is checked in the game (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_loot::*;

fn catalog() -> ItemCatalog {
    ItemCatalog(vec![
        ItemDefinition::new("kit", "Health kit", ItemKind::Heal(50), 3),
        ItemDefinition::new(
            "rockets",
            "Rockets",
            ItemKind::Ammo(WeaponKind::Launcher),
            8,
        ),
    ])
}

fn config() -> LootConfig {
    LootConfig {
        kinds: vec![PickupKind {
            kind: "health".to_string(),
            contents: vec![(ItemId::new("kit"), 1)],
            respawn_seconds: 1.0,
            look: PickupLook {
                color: Color::srgb(0.1, 0.9, 0.2),
                size: Vec3::splat(0.4),
            },
            sound: Some(SoundCue::new("sounds/kit.wav", 0.5)),
        }],
        drop_seconds: 2.0,
        ..LootConfig::default()
    }
}

/// A health spot at the origin and a spot of a kind the config does not know.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<SoundEvents>()
        .insert_resource(catalog())
        .insert_resource(config())
        .add_plugins(LootModulePlugin);
    app.world_mut().resource_mut::<PickupSpots>().0 = [("first", "health"), ("other", "unknown")]
        .map(|(name, kind)| PickupSpot {
            name: name.to_string(),
            kind: kind.to_string(),
            feet: if name == "first" {
                Vec3::ZERO
            } else {
                Vec3::X * 20.0
            },
        })
        .to_vec();
    ticks(&mut app, 1);
    app
}

fn player(app: &mut App, id: u32, feet: Vec3) -> Entity {
    let size = CharacterSize::default();
    app.world_mut()
        .spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            Health::full(100),
            Inventory::new(4),
            Transform::from_translation(size.center(feet)),
        ))
        .id()
}

fn ticks(app: &mut App, count: u32) {
    for _ in 0..count {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn ok(app: &mut App, line: &str) {
    run_command_line(app.world_mut(), line)
        .unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn grants(app: &mut App) -> Vec<(u32, String, u32)> {
    app.world_mut()
        .resource_mut::<Messages<GrantRequested>>()
        .drain()
        .map(|grant| (grant.player.0, grant.item.0, grant.count))
        .collect()
}

fn lose(app: &mut App, player: u32, items: &[(&str, u32)]) {
    app.world_mut().write_message(ItemsLost {
        player: PlayerId(player),
        items: items
            .iter()
            .map(|(item, count)| ItemStack {
                item: ItemId::new(item),
                count: *count,
            })
            .collect(),
        at: CharacterSize::default().center(Vec3::Z * 5.0),
    });
}

#[test]
fn walking_over_a_pickup_takes_it_and_it_comes_back() {
    let mut app = app();
    ok(&mut app, "assert.loot_available first true");
    assert!(run_command_line(app.world_mut(), "assert.loot_available other true").is_err());
    player(&mut app, 2, Vec3::new(0.5, 0.0, 0.0));
    player(&mut app, 1, Vec3::new(-0.5, 0.0, 0.0));
    ticks(&mut app, 1);
    assert_eq!(
        grants(&mut app),
        [(1, "kit".to_string(), 1)],
        "one, the first id"
    );
    ok(&mut app, "assert.loot_available first false");
    ticks(&mut app, 63);
    assert!(grants(&mut app).is_empty());
    ticks(&mut app, 1);
    assert_eq!(
        grants(&mut app).len(),
        1,
        "back after a second, taken at once"
    );
}

#[test]
fn it_stays_for_whoever_cannot_use_it_is_dead_far_or_on_another_level() {
    let mut app = app();
    let full = player(&mut app, 1, Vec3::ZERO);
    app.world_mut()
        .get_mut::<Inventory>(full)
        .unwrap()
        .add(&catalog().0[0], 3);
    let dead = player(&mut app, 2, Vec3::ZERO);
    app.world_mut()
        .get_mut::<Health>(dead)
        .unwrap()
        .apply_damage(100);
    player(&mut app, 3, Vec3::X * 1.2);
    player(&mut app, 4, Vec3::Y * 2.0);
    ticks(&mut app, 3);
    assert!(grants(&mut app).is_empty());
    ok(&mut app, "assert.loot_available first true");
}

#[test]
fn a_finished_match_and_a_client_take_nothing() {
    let mut app = app();
    player(&mut app, 1, Vec3::ZERO);
    let finished = app
        .world_mut()
        .spawn(MatchState {
            phase: MatchPhase::Finished {
                winner: None,
                ticks_until_restart: 10,
            },
        })
        .id();
    ticks(&mut app, 2);
    assert!(grants(&mut app).is_empty());
    app.world_mut().despawn(finished);
    app.world_mut().insert_resource(NetLocalClient(Some(1)));
    ticks(&mut app, 2);
    assert!(grants(&mut app).is_empty());
}

#[test]
fn a_drop_gives_what_fits_keeps_the_rest_and_ends() {
    let mut app = app();
    lose(&mut app, 7, &[("rockets", 12), ("kit", 2)]);
    ticks(&mut app, 1);
    ok(&mut app, "assert.loot_drops 1");
    player(&mut app, 1, Vec3::Z * 5.0);
    ticks(&mut app, 1);
    assert_eq!(
        grants(&mut app),
        [(1, "rockets".to_string(), 8), (1, "kit".to_string(), 2)]
    );
    ok(&mut app, "assert.loot_drops 1");
    // The grants are read by nobody here, so the player could take again: it leaves first.
    let mut transforms = app.world_mut().query::<&mut Transform>();
    for mut transform in transforms.iter_mut(app.world_mut()) {
        transform.translation.x += 30.0;
    }
    ticks(&mut app, 128);
    ok(&mut app, "assert.loot_drops 0");
}

#[test]
fn a_new_match_fills_the_spots_and_removes_the_drops() {
    let mut app = app();
    player(&mut app, 1, Vec3::ZERO);
    ticks(&mut app, 1);
    lose(&mut app, 1, &[("kit", 1)]);
    ticks(&mut app, 1);
    ok(&mut app, "assert.loot_drops 1");
    app.world_mut().write_message(MatchRestarted);
    ticks(&mut app, 1);
    ok(&mut app, "assert.loot_available first true");
    let mut gone = app
        .world_mut()
        .query_filtered::<&Pickup, With<RemoveNetEntity>>();
    assert_eq!(gone.iter(app.world()).count(), 1);
}

#[test]
fn a_pickup_is_drawn_while_it_is_there() {
    let mut app = app();
    app.world_mut().run_schedule(Update);
    let mut drawn = app.world_mut().query::<(&BoxVisual, Has<HideVisuals>)>();
    assert_eq!(
        drawn.single(app.world()).map(|(_, hidden)| hidden).ok(),
        Some(false)
    );
    player(&mut app, 1, Vec3::ZERO);
    ticks(&mut app, 1);
    app.world_mut().run_schedule(Update);
    assert_eq!(
        drawn.single(app.world()).map(|(_, hidden)| hidden).ok(),
        Some(true)
    );
}

#[test]
#[should_panic(expected = "no item")]
fn contents_the_catalog_does_not_know_are_refused_at_startup() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(config())
        .add_plugins(LootModulePlugin);
}

#[test]
fn the_tick_of_a_respawn_takes_nothing_where_the_player_died() {
    let mut app = app();
    player(&mut app, 1, Vec3::ZERO);
    app.world_mut().write_message(PlayerRespawned {
        player: PlayerId(1),
        spawn: SpawnPoint {
            feet: Vec3::X * 9.0,
            yaw: 0.0,
        },
    });
    ticks(&mut app, 1);
    assert!(grants(&mut app).is_empty());
}

#[test]
fn a_pickup_is_recorded_on_its_taker_and_heard_once() {
    let mut app = app();
    let taker = player(&mut app, 1, Vec3::X * 9.0);
    ticks(&mut app, 2);
    let place = CharacterSize::default().center(Vec3::ZERO);
    app.world_mut()
        .get_mut::<Transform>(taker)
        .unwrap()
        .translation = place;
    ticks(&mut app, 3);
    let record = app.world().get::<LastPickup>(taker).unwrap();
    assert_eq!((record.count, record.kind.as_deref()), (1, Some("health")));
    assert_eq!(record.items[0].item, ItemId::new("kit"));
    let heard: Vec<String> = app
        .world()
        .resource::<SoundEvents>()
        .requests
        .iter()
        .map(|event| event.id.clone())
        .collect();
    assert_eq!(heard, ["loot/1/1"]);
}
