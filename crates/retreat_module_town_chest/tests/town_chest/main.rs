//! The town chest with only Bevy and the contract crates: it opens only at a chest spot in a safe
//! area, keeps its items in its record while closed, closes when the character walks away, grows
//! its rows, can be shared by an account, and refuses moves outside the city. A hand-made bag
//! stands in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, RecordKey, RecordStore, RemoveNetEntity, SimulationRng,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_town_chest::*;

fn catalog() -> GridItemCatalog {
    GridItemCatalog {
        bases: vec![ItemBase {
            id: "sword".to_string(),
            name: "Sword".to_string(),
            size: UVec2::new(1, 3),
            slots: Vec::new(),
            two_handed: false,
            max_stack: 1,
            rollable: true,
            affix_pool: Vec::new(),
            icon: None,
            color: [0.5; 3],
        }],
        rarities: vec![ItemRarity {
            id: "common".to_string(),
            name: "Common".to_string(),
            weight: 1,
            affixes: (0, 0),
            color: [1.0; 3],
        }],
        affixes: Vec::new(),
    }
}

fn character() -> PlayingCharacter {
    PlayingCharacter {
        id: CharacterId("ada-1".to_string()),
        name: "Ada".to_string(),
        account: AccountId("ada".to_string()),
    }
}

fn app_with(config: TownChestConfig) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .insert_resource(SimulationRng::from_seed(1))
        .insert_resource(catalog())
        .insert_resource(config)
        .add_plugins(TownChestModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut()
        .insert_resource(ServiceSpots(vec![ServiceSpot {
            kind: CHEST_SPOT_KIND.to_string(),
            position: Vec3::ZERO,
            radius: 2.0,
        }]));
    app
}

fn app() -> App {
    app_with(TownChestConfig::default())
}

fn bag_id() -> ContainerId {
    container_id(ContainerKind::Bag, &character().id)
}

fn chest_id() -> ContainerId {
    container_id(ContainerKind::Chest, &character().id)
}

/// A playing body at `x` and a hand-made bag with `swords` swords.
fn start(app: &mut App, x: f32, swords: usize) -> (Entity, Vec<ItemUid>) {
    let body = app
        .world_mut()
        .spawn((
            Name::new("character_ada-1_1"),
            PlayerId(0),
            character(),
            Transform::from_xyz(x, 1.0, 0.0),
        ))
        .id();
    spawn_container(
        app.world_mut(),
        ItemContainer {
            id: bag_id(),
            kind: ContainerKind::Bag,
            title: "Bag".to_string(),
            layout: ContainerLayout::Grid {
                columns: 6,
                rows: 6,
                visible_rows: 0,
            },
            character: character().id,
            only_in_safe_area: false,
            drop_command: "test_bag.put".to_string(),
            record_owner: "character/ada-1".to_string(),
            record_part: "test_bag".to_string(),
        },
        PlayerId(0),
    );
    let sword = GridItem {
        uid: ItemUid(0),
        base: "sword".to_string(),
        rarity: "common".to_string(),
        affixes: Vec::new(),
        level_requirement: 1,
        quantity: 1,
        size: UVec2::new(1, 3),
    };
    let uids = add_to_container(
        app.world_mut(),
        PlayerId(0),
        &bag_id(),
        NewItems {
            unique: vec![sword; swords],
            stack: None,
        },
        Vec::new(),
    )
    .unwrap();
    (body, uids)
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
    let removed: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<RemoveNetEntity>>()
        .iter(app.world())
        .collect();
    for entity in removed {
        app.world_mut().despawn(entity);
    }
}

#[test]
fn the_chest_opens_only_at_a_chest_spot() {
    let mut app = app();
    let (body, _) = start(&mut app, 5.0, 0);
    let refusal = run(&mut app, "chest.open").unwrap_err().to_string();
    assert!(refusal.contains("walk up to a town chest"), "{refusal}");
    app.world_mut()
        .get_mut::<Transform>(body)
        .unwrap()
        .translation
        .x = 1.0;
    ok(&mut app, "chest.open");
    ok(&mut app, "assert.chest_open");
    ok(&mut app, "chest.close");
    tick(&mut app);
    ok(&mut app, "assert.chest_closed");
}

#[test]
fn stored_items_stay_in_the_record_and_come_back_and_walking_away_closes_it() {
    let mut app = app();
    let (body, uids) = start(&mut app, 0.0, 2);
    ok(&mut app, "chest.open");
    ok(&mut app, &format!("chest.put {} 3 0 1", uids[0].0));
    ok(&mut app, "assert.chest_items 1");
    app.world_mut()
        .get_mut::<Transform>(body)
        .unwrap()
        .translation
        .x = 9.0;
    tick(&mut app);
    ok(&mut app, "assert.chest_closed");
    assert!(
        placed_item(app.world_mut(), uids[0]).is_none(),
        "a closed chest's items are only saved"
    );
    app.world_mut()
        .get_mut::<Transform>(body)
        .unwrap()
        .translation
        .x = 0.5;
    ok(&mut app, "chest.open");
    let stored = placed_item(app.world_mut(), uids[0]).expect("back from the record");
    assert_eq!(
        (stored.place.x, stored.place.y, stored.place.rotated),
        (3, 0, true)
    );
    // Back into the bag.
    move_item(
        app.world_mut(),
        PlayerId(0),
        uids[0],
        &bag_id(),
        5,
        0,
        false,
    )
    .unwrap();
    ok(&mut app, "assert.chest_items 0");
}

#[test]
fn rows_grow_below_the_lowest_item_so_the_chest_has_no_limit() {
    let mut app = app();
    let (_, uids) = start(&mut app, 0.0, 1);
    ok(&mut app, "chest.open");
    let rows = |app: &mut App| match container_of(app.world_mut(), &chest_id()).unwrap().layout {
        ContainerLayout::Grid { rows, .. } => rows,
        ContainerLayout::Equipment => unreachable!(),
    };
    assert_eq!(rows(&mut app), 8);
    // The last free rows (5..8 for a 3-high sword at row 5): the chest grows to 10.
    ok(&mut app, &format!("chest.put {} 0 5 0", uids[0].0));
    tick(&mut app);
    assert_eq!(rows(&mut app), 10);
}

#[test]
fn outside_a_safe_area_the_chest_neither_opens_nor_takes_items() {
    let mut app = app();
    app.world_mut().insert_resource(MapAreas(vec![Area {
        id: AreaId("dungeon".to_string()),
        name: "Dungeon".to_string(),
        rules: AreaRules {
            safe: false,
            pvp: true,
            tier: 1,
            max_entry_level: None,
        },
        min: Vec2::splat(-50.0),
        max: Vec2::splat(50.0),
        home: false,
    }]));
    let (body, _) = start(&mut app, 0.0, 1);
    app.world_mut()
        .entity_mut(body)
        .insert(InArea(AreaId("dungeon".to_string())));
    assert!(run(&mut app, "chest.open").is_err());
    ok(&mut app, "assert.chest_closed");
}

#[test]
fn a_shared_chest_is_saved_under_the_account() {
    let mut app = app_with(TownChestConfig {
        shared_by_account: true,
        ..Default::default()
    });
    let (_, uids) = start(&mut app, 0.0, 1);
    ok(&mut app, "chest.open");
    ok(&mut app, &format!("chest.put {} 0 0 0", uids[0].0));
    let store = app.world().resource::<RecordStore>();
    assert!(
        store
            .read(&RecordKey::new("account/ada", CHEST_RECORD_PART))
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .read(&RecordKey::new("character/ada-1", CHEST_RECORD_PART))
            .unwrap()
            .is_none()
    );
}
