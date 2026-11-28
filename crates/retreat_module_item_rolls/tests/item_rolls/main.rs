//! Item rolls with only Bevy and the contract crates: grants land in the bag with new unique uids
//! that survive a restart, a full bag gets nothing, the same seed rolls the same items, and test
//! commands reach the server only when allowed. A hand-made bag stands in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, NetAllowedCommands, RecordStore, SimulationRng,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_item_rolls::*;

fn catalog() -> GridItemCatalog {
    GridItemCatalog {
        bases: vec![
            ItemBase {
                id: "sword".to_string(),
                name: "Sword".to_string(),
                size: UVec2::new(1, 2),
                slots: vec![EquipSlot::MainHand],
                two_handed: false,
                max_stack: 1,
                rollable: true,
                affix_pool: vec!["strength".to_string(), "speed".to_string()],
                icon: None,
                color: [0.5; 3],
            },
            ItemBase {
                id: "gold".to_string(),
                name: "Gold".to_string(),
                size: UVec2::ONE,
                slots: Vec::new(),
                two_handed: false,
                max_stack: 100,
                rollable: false,
                affix_pool: Vec::new(),
                icon: None,
                color: [0.9, 0.8, 0.2],
            },
        ],
        rarities: vec![
            ItemRarity {
                id: "common".to_string(),
                name: "Common".to_string(),
                weight: 1,
                affixes: (0, 0),
                color: [1.0; 3],
            },
            ItemRarity {
                id: "rare".to_string(),
                name: "Rare".to_string(),
                weight: 1,
                affixes: (2, 2),
                color: [1.0, 1.0, 0.3],
            },
        ],
        affixes: ["strength", "speed"]
            .iter()
            .map(|id| ItemAffix {
                id: id.to_string(),
                text: format!("+{{}} {id}"),
                min: 1,
                max: 5,
                per_level: 1,
            })
            .collect(),
    }
}

fn character() -> PlayingCharacter {
    PlayingCharacter {
        id: CharacterId("ada-1".to_string()),
        name: "Ada".to_string(),
        account: AccountId("ada".to_string()),
    }
}

fn app(config: ItemRollsConfig, store: RecordStore, seed: u64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(store)
        .insert_resource(SimulationRng::from_seed(seed))
        .insert_resource(catalog())
        .insert_resource(config)
        .add_plugins(ItemRollsModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut().spawn((
        Name::new("character_ada-1_1"),
        PlayerId(0),
        character(),
        Transform::default(),
    ));
    // Opened from its record, as the bag module does (a restart finds the saved bag).
    open_container(
        app.world_mut(),
        ItemContainer {
            id: bag_id(),
            kind: ContainerKind::Bag,
            title: "Bag".to_string(),
            layout: ContainerLayout::Grid {
                columns: 4,
                rows: 2,
                visible_rows: 0,
            },
            character: character().id,
            only_in_safe_area: false,
            drop_command: "test_bag.put".to_string(),
            record_owner: "character/ada-1".to_string(),
            record_part: "test_bag".to_string(),
        },
        PlayerId(0),
    )
    .unwrap();
    app
}

fn bag_id() -> ContainerId {
    container_id(ContainerKind::Bag, &character().id)
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn bag(app: &mut App) -> Vec<PlacedItem> {
    container_items(app.world_mut(), &bag_id())
}

#[test]
fn a_chosen_item_rolls_its_rarity_affixes_and_requirement() {
    let mut app = app(ItemRollsConfig::default(), RecordStore::in_memory(), 1);
    ok(&mut app, "items.grant sword rare 4");
    let items = bag(&mut app);
    assert_eq!(items.len(), 1);
    let sword = &items[0].item;
    assert_eq!(
        (sword.rarity.as_str(), sword.level_requirement),
        ("rare", 4)
    );
    assert_eq!(sword.affixes.len(), 2);
    assert!(
        sword
            .affixes
            .iter()
            .all(|affix| (4..=8).contains(&affix.value)),
        "{sword:?}"
    );
    let unknown = run(&mut app, "items.grant axe rare 1")
        .unwrap_err()
        .to_string();
    assert!(
        unknown.contains("unknown item base `axe`") && unknown.contains("sword"),
        "{unknown}"
    );
}

#[test]
fn the_same_seed_rolls_the_same_items() {
    let roll = |seed| {
        let mut app = app(ItemRollsConfig::default(), RecordStore::in_memory(), seed);
        ok(&mut app, "items.grant_test 3");
        bag(&mut app)
            .into_iter()
            .map(|p| p.item)
            .collect::<Vec<_>>()
    };
    assert_eq!(roll(7), roll(7));
    assert!(
        roll(7)
            .iter()
            .all(|item| item.base == "sword" && (1..=10).contains(&item.level_requirement))
    );
}

#[test]
fn a_full_bag_gets_nothing_and_says_so() {
    let mut app = app(ItemRollsConfig::default(), RecordStore::in_memory(), 1);
    ok(&mut app, "items.grant_test 4");
    assert_eq!(bag(&mut app).len(), 4);
    assert!(refusal(&mut app, "items.grant_test 1").contains("full"));
    assert_eq!(bag(&mut app).len(), 4);
}

#[test]
fn uids_stay_unique_across_a_restart() {
    let mut first = app(ItemRollsConfig::default(), RecordStore::in_memory(), 1);
    ok(&mut first, "items.grant_gold 150");
    let before: Vec<u64> = bag(&mut first).iter().map(|p| p.item.uid.0).collect();
    // A restart: the same records, a new process.
    let store = first.world_mut().remove_resource::<RecordStore>().unwrap();
    let mut second = app(ItemRollsConfig::default(), store, 1);
    ok(&mut second, "items.grant sword common 1");
    let after = bag(&mut second)
        .iter()
        .find(|placed| placed.item.base == "sword")
        .expect("the sword was granted")
        .item
        .uid
        .0;
    assert!(
        !before.contains(&after),
        "{after} was handed out before: {before:?}"
    );
    assert!(after > *before.iter().max().unwrap());
}

#[test]
fn clients_may_send_test_commands_only_when_tests_are_on() {
    let allowed = |test_commands| {
        let app = app(
            ItemRollsConfig {
                test_commands,
                ..Default::default()
            },
            RecordStore::in_memory(),
            1,
        );
        app.world()
            .resource::<NetAllowedCommands>()
            .0
            .contains("items.grant_test")
    };
    assert!(!allowed(false));
    assert!(allowed(true));
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
