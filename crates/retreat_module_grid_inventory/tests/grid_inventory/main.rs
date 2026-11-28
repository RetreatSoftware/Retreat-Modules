//! The bag with only Bevy and the contract crates: it opens with its saved items when a character
//! plays and closes when it stops, moves turn and stack items, refusals reach the player, splits,
//! and a network client only sends its request. With the real engine it is checked in the crawler
//! testbed (Dungeon-Crawler).

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, NetInputQueue, RecordStore, RemoveNetEntity, SimulationRng,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_grid_inventory::*;

fn catalog() -> GridItemCatalog {
    let base = |id: &str, w: u32, h: u32, max_stack: u32| ItemBase {
        id: id.to_string(),
        name: id.to_string(),
        size: UVec2::new(w, h),
        slots: Vec::new(),
        two_handed: false,
        max_stack,
        rollable: true,
        affix_pool: Vec::new(),
        icon: None,
        color: [0.5; 3],
    };
    GridItemCatalog {
        bases: vec![
            base("sword", 1, 3, 1),
            base("shield", 2, 2, 1),
            base("gold", 1, 1, 100),
        ],
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

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .insert_resource(SimulationRng::from_seed(1))
        .insert_resource(catalog())
        .insert_resource(GridInventoryConfig {
            columns: 4,
            rows: 3,
        })
        .add_plugins(GridInventoryModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app
}

fn spawn_body(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Name::new("character_ada-1_1"),
            PlayerId(0),
            character(),
            Transform::default(),
        ))
        .id()
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn bag_id() -> ContainerId {
    container_id(ContainerKind::Bag, &character().id)
}

fn give(app: &mut App, unique: &[&str], gold: u32) -> Vec<ItemUid> {
    let catalog = catalog();
    let items = unique
        .iter()
        .map(|base| GridItem {
            uid: ItemUid(0),
            base: base.to_string(),
            rarity: "common".to_string(),
            affixes: Vec::new(),
            level_requirement: 1,
            quantity: 1,
            size: catalog.base(base).unwrap().size,
        })
        .collect();
    let stack = (gold > 0).then(|| ("gold".to_string(), gold));
    add_to_container(
        app.world_mut(),
        PlayerId(0),
        &bag_id(),
        NewItems {
            unique: items,
            stack,
        },
        Vec::new(),
    )
    .expect("fits")
}

fn bag(app: &mut App) -> Vec<PlacedItem> {
    container_items(app.world_mut(), &bag_id())
}

/// The engine retires removed entities; without it, despawn them.
fn despawn_removed(app: &mut App) {
    let world = app.world_mut();
    let removed: Vec<Entity> = world
        .query_filtered::<Entity, With<RemoveNetEntity>>()
        .iter(world)
        .collect();
    for entity in removed {
        world.despawn(entity);
    }
}

#[test]
fn the_bag_opens_with_its_saved_items_and_closes_when_the_character_stops() {
    let mut app = app();
    let body = spawn_body(&mut app);
    tick(&mut app);
    let container = container_of(app.world_mut(), &bag_id()).expect("open");
    assert_eq!(
        container.layout,
        ContainerLayout::Grid {
            columns: 4,
            rows: 3,
            visible_rows: 0
        }
    );
    give(&mut app, &["sword", "shield"], 0);
    assert_eq!(bag(&mut app).len(), 2);
    app.world_mut().despawn(body);
    tick(&mut app);
    despawn_removed(&mut app);
    assert!(container_entity(app.world_mut(), &bag_id()).is_none());
    assert!(bag(&mut app).is_empty());
    spawn_body(&mut app);
    tick(&mut app);
    let back: Vec<String> = bag(&mut app).into_iter().map(|p| p.item.base).collect();
    assert_eq!(back, ["sword", "shield"]);
}

#[test]
fn a_move_turns_an_item_and_a_blocked_one_tells_the_player() {
    let mut app = app();
    let body = spawn_body(&mut app);
    tick(&mut app);
    let uids = give(&mut app, &["sword", "shield"], 0);
    // The sword stands at (0, 0); the shield at (1, 0).
    ok(&mut app, &format!("bag.put {} 1 2 1", uids[0].0));
    let sword = placed_item(app.world_mut(), uids[0]).unwrap();
    assert_eq!(
        (sword.place.x, sword.place.y, sword.place.rotated),
        (1, 2, true)
    );
    let refusal = run(&mut app, &format!("bag.put {} 2 1 0", uids[0].0))
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("does not fit"), "{refusal}");
    let notice = app.world().get::<ItemNotice>(body).unwrap();
    assert!(notice.text.contains("does not fit"));
    // Saved: the record holds the turned sword.
    let store = app.world().resource::<RecordStore>();
    let record = store
        .read(&retreat_engine_api::RecordKey::new(
            "character/ada-1",
            BAG_RECORD_PART,
        ))
        .unwrap()
        .unwrap();
    let saved = items_from_record(&record.data, &bag_id()).unwrap();
    assert!(
        saved
            .iter()
            .any(|p| p.item.uid == uids[0] && p.place.rotated)
    );
}

#[test]
fn gold_stacks_merge_on_a_drop_and_split_off() {
    let mut app = app();
    spawn_body(&mut app);
    tick(&mut app);
    give(&mut app, &[], 150);
    let stacks: Vec<PlacedItem> = bag(&mut app);
    assert_eq!(
        stacks.iter().map(|p| p.item.quantity).collect::<Vec<_>>(),
        [100, 50]
    );
    let (full, half) = (stacks[0].clone(), stacks[1].clone());
    // Splitting 20 off the half stack makes a third stack.
    ok(&mut app, &format!("bag.split {} 20", half.item.uid.0));
    let quantities: Vec<u32> = bag(&mut app).iter().map(|p| p.item.quantity).collect();
    assert_eq!(quantities, [100, 30, 20]);
    ok(&mut app, "assert.bag_quantity gold 150");
    // Dropping the 20 on the 30 merges them; the full stack takes nothing.
    let twenty = bag(&mut app)[2].clone();
    ok(
        &mut app,
        &format!(
            "bag.put {} {} {} 0",
            twenty.item.uid.0, half.place.x, half.place.y
        ),
    );
    despawn_removed(&mut app);
    let quantities: Vec<u32> = bag(&mut app).iter().map(|p| p.item.quantity).collect();
    assert_eq!(quantities, [100, 50]);
    assert!(
        run(
            &mut app,
            &format!(
                "bag.put {} {} {} 0",
                half.item.uid.0, full.place.x, full.place.y
            )
        )
        .unwrap_err()
        .to_string()
        .contains("stack is full")
    );
    ok(&mut app, "assert.bag_items 2");
}

#[test]
fn a_client_only_sends_its_request_to_the_server() {
    let mut app = app();
    spawn_body(&mut app);
    tick(&mut app);
    let uids = give(&mut app, &["sword"], 0);
    app.world_mut().resource_mut::<NetInputQueue>().accepting = true;
    ok(&mut app, &format!("bag.put {} 2 0 0", uids[0].0));
    let sent = app.world().resource::<NetInputQueue>().lines.clone();
    assert_eq!(sent, [format!("bag.put {} 2 0 0", uids[0].0)]);
    let sword = placed_item(app.world_mut(), uids[0]).unwrap();
    assert_eq!(
        (sword.place.x, sword.place.y),
        (0, 0),
        "the client moved nothing"
    );
}
