//! The town chest with only Bevy and the contract crates: it opens only at a chest spot in a safe
//! area, keeps its items in its record while closed, closes when the character walks away, grows
//! its rows, is one chest for all of an account's characters in every world (two apps on one
//! shared store are two world servers), and refuses moves outside the city. A hand-made bag
//! stands in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, RecordKey, RecordStore, RemoveNetEntity, RetiredEntity,
    SharedMemoryRecordBackend, SimulationRng, run_command_line,
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
    character_of_ada(1, "Ada")
}

/// Character `ada-<number>` of the account `ada`.
fn character_of_ada(number: u32, name: &str) -> PlayingCharacter {
    PlayingCharacter {
        id: CharacterId(format!("ada-{number}")),
        name: name.to_string(),
        account: AccountId("ada".to_string()),
    }
}

fn app_with(config: TownChestConfig) -> App {
    app_on(config, RecordStore::in_memory())
}

/// A world server whose records are `store` (several apps on one shared store are several worlds).
fn app_on(config: TownChestConfig, store: RecordStore) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(store)
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
    start_as(app, PlayerId(0), character(), x, swords)
}

/// `playing`, played by `player`, at `x` with a hand-made bag of `swords` swords.
fn start_as(
    app: &mut App,
    player: PlayerId,
    playing: PlayingCharacter,
    x: f32,
    swords: usize,
) -> (Entity, Vec<ItemUid>) {
    let bag = container_id(ContainerKind::Bag, &playing.id);
    let body = app
        .world_mut()
        .spawn((
            Name::new(format!("character_{}_1", playing.id.0)),
            player,
            playing.clone(),
            Transform::from_xyz(x, 1.0, 0.0),
        ))
        .id();
    spawn_container(
        app.world_mut(),
        ItemContainer {
            id: bag.clone(),
            kind: ContainerKind::Bag,
            title: "Bag".to_string(),
            layout: ContainerLayout::Grid {
                columns: 6,
                rows: 6,
                visible_rows: 0,
            },
            character: playing.id.clone(),
            only_in_safe_area: false,
            drop_command: "test_bag.put".to_string(),
            record_owner: character_record_owner(&playing.id),
            record_part: "test_bag".to_string(),
        },
        player,
        None,
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
        player,
        &bag,
        NewItems {
            unique: vec![sword; swords],
            stack: None,
        },
        ExtraRecords::default(),
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
    let refusal = refusal(&mut app, "chest.open");
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
    assert!(refusal(&mut app, "chest.open").contains("walk up to a town chest"));
    ok(&mut app, "assert.chest_closed");
}

/// What the engine does with a removed entity: keeps it a while, retired.
fn retire_removed(app: &mut App) {
    let removed: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<RemoveNetEntity>>()
        .iter(app.world())
        .collect();
    for entity in removed {
        app.world_mut()
            .entity_mut(entity)
            .remove::<RemoveNetEntity>()
            .insert(RetiredEntity {
                since: std::time::Duration::ZERO,
            });
    }
}

fn ticks(app: &mut App, count: usize) {
    for _ in 0..count {
        tick(app);
    }
}

/// Enough ticks for an open chest to be read again (the default half second at 64 Hz is 32).
const REFRESH_TICKS: usize = 40;

/// The uids saved in a record of the contract's container format.
fn saved_uids(store: &RecordStore, owner: &str, part: &str) -> Vec<u64> {
    store
        .read(&RecordKey::new(owner, part))
        .unwrap()
        .map(|record| {
            record.data["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["uid"].as_u64().unwrap())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn two_worlds_with_the_chest_open_at_once_never_lose_or_copy_an_item() {
    // Ada plays in world one, Ann (same account) in world two, at the same time.
    let shared = SharedMemoryRecordBackend::default();
    let config = TownChestConfig::default();
    let mut one = app_on(config.clone(), RecordStore::new(Box::new(shared.clone())));
    let mut two = app_on(config, RecordStore::new(Box::new(shared.clone())));
    let (_, ada) = start_as(&mut one, PlayerId(0), character_of_ada(1, "Ada"), 0.0, 1);
    let (_, ann) = start_as(&mut two, PlayerId(0), character_of_ada(2, "Ann"), 0.0, 1);
    assert_ne!(ada[0], ann[0], "uids are unique across worlds");
    ok(&mut one, "chest.open");
    ok(&mut two, "chest.open");
    // Ada stores her sword; Ann's window has not been read again yet and she drops on that spot.
    ok(&mut one, &format!("chest.put {} 0 0 0", ada[0].0));
    let why = refusal(&mut two, &format!("chest.put {} 0 0 0", ann[0].0));
    assert!(
        why.contains("changed by another of your characters"),
        "{why}"
    );
    let ann_chest = container_id(ContainerKind::Chest, &CharacterId("ada-2".to_string()));
    let ann_bag = container_id(ContainerKind::Bag, &CharacterId("ada-2".to_string()));
    assert_eq!(
        placed_item(two.world_mut(), ann[0])
            .unwrap()
            .place
            .container,
        ann_bag
    );
    // The refusal showed her the chest as it is now: Ada's sword.
    assert_eq!(
        placed_item(two.world_mut(), ada[0])
            .unwrap()
            .place
            .container,
        ann_chest
    );
    ok(&mut two, &format!("chest.put {} 1 0 0", ann[0].0));
    ok(&mut two, "assert.chest_items 2");
    // Ada's open window shows Ann's sword after a moment.
    ticks(&mut one, REFRESH_TICKS);
    ok(&mut one, "assert.chest_items 2");
    // Ann takes Ada's sword into her bag; Ada's window loses it.
    move_item(two.world_mut(), PlayerId(0), ada[0], &ann_bag, 3, 0, false).unwrap();
    ticks(&mut one, REFRESH_TICKS);
    ok(&mut one, "assert.chest_items 1");
    assert!(
        placed_item(one.world_mut(), ada[0]).is_none(),
        "not in world one any more"
    );
    // Every item is saved exactly once.
    let store = RecordStore::new(Box::new(shared));
    assert_eq!(
        saved_uids(&store, "account/ada", CHEST_RECORD_PART),
        [ann[0].0]
    );
    assert_eq!(
        saved_uids(&store, "character/ada-2", "test_bag"),
        [ada[0].0]
    );
    assert!(saved_uids(&store, "character/ada-1", "test_bag").is_empty());
}

#[test]
fn every_character_of_the_account_opens_the_same_chest() {
    let mut app = app();
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
    // Ann, another character of the account, standing at the chest too (a test only: in one world
    // an account plays one character at a time) cannot open the same chest a second time here.
    start_as(&mut app, PlayerId(1), character_of_ada(2, "Ann"), 0.5, 0);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(1))));
    let why = refusal(&mut app, "chest.open");
    assert!(why.contains("open for ada-1"), "{why}");
    // Once Ada closed it, Ann opens it and finds Ada's sword. The engine keeps a removed entity a
    // while as retired (here by hand), and a retired chest does not count as open.
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    ok(&mut app, "chest.close");
    retire_removed(&mut app);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(1))));
    ok(&mut app, "chest.open");
    ok(&mut app, "assert.chest_items 1");
}
/// Runs a player request that must be refused and returns the notice the player got (a refusal is
/// the player's notice, not an error).
fn refusal(app: &mut App, line: &str) -> String {
    let before = notice(app).map_or(0, |notice| notice.serial);
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
    let after = notice(app).unwrap_or_else(|| panic!("`{line}` told the player nothing"));
    assert!(after.serial > before, "`{line}` was not refused");
    after.text
}

fn notice(app: &mut App) -> Option<ItemNotice> {
    let world = app.world_mut();
    world.query::<&ItemNotice>().iter(world).next().cloned()
}
