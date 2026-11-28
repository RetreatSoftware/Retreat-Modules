//! The wallet with only Bevy and the contract crates: deposits and withdrawals at a deposit spot,
//! one atomic save of bag and balance, refusals, and the balance coming back. A hand-made bag stands
//! in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, RecordKey, RecordStore, RemoveNetEntity, SimulationRng,
    run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_wallet::*;

fn catalog() -> GridItemCatalog {
    GridItemCatalog {
        bases: vec![ItemBase {
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

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(RecordStore::in_memory())
        .insert_resource(SimulationRng::from_seed(1))
        .insert_resource(catalog())
        .add_plugins(WalletModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut()
        .insert_resource(ServiceSpots(vec![ServiceSpot {
            kind: DEPOSIT_SPOT_KIND.to_string(),
            position: Vec3::ZERO,
            radius: 2.0,
        }]));
    app
}

fn bag_id() -> ContainerId {
    container_id(ContainerKind::Bag, &character().id)
}

/// A playing body at `x` with a hand-made 2 x 2 bag holding `gold`.
fn start(app: &mut App, x: f32, gold: u32) -> Entity {
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
                columns: 2,
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
    );
    if gold > 0 {
        add_to_container(
            app.world_mut(),
            PlayerId(0),
            &bag_id(),
            NewItems {
                unique: Vec::new(),
                stack: Some(("gold".to_string(), gold)),
            },
            Vec::new(),
        )
        .unwrap();
    }
    tick(app);
    body
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

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

fn refusal(app: &mut App, line: &str) -> String {
    run(app, line)
        .err()
        .unwrap_or_else(|| panic!("`{line}` must be refused"))
        .to_string()
}

fn carried(app: &mut App) -> u32 {
    container_items(app.world_mut(), &bag_id())
        .iter()
        .map(|placed| placed.item.quantity)
        .sum()
}

fn saved(app: &App, part: &str) -> retreat_engine_api::serde_json::Value {
    app.world()
        .resource::<RecordStore>()
        .read(&RecordKey::new("character/ada-1", part))
        .unwrap()
        .expect("saved")
        .data
}

#[test]
fn a_deposit_moves_all_carried_gold_into_the_balance_in_one_save() {
    let mut app = app();
    start(&mut app, 0.0, 150);
    ok(&mut app, "assert.wallet_balance 0");
    ok(&mut app, "wallet.deposit");
    tick(&mut app);
    ok(&mut app, "assert.wallet_balance 150");
    assert_eq!(carried(&mut app), 0);
    assert_eq!(saved(&app, WALLET_RECORD_PART)["balance"], 150);
    assert_eq!(
        saved(&app, "test_bag")["items"].as_array().unwrap().len(),
        0
    );
    assert!(refusal(&mut app, "wallet.deposit").contains("carry no gold"));
}

#[test]
fn a_withdrawal_needs_the_balance_and_room_in_the_bag() {
    let mut app = app();
    start(&mut app, 0.0, 300);
    ok(&mut app, "wallet.deposit");
    tick(&mut app);
    assert!(refusal(&mut app, "wallet.withdraw 301").contains("only 300"));
    // The 2 x 2 bag holds four stacks of 100 at most.
    ok(&mut app, "wallet.withdraw 120");
    ok(&mut app, "assert.wallet_balance 180");
    assert_eq!(carried(&mut app), 120);
    assert_eq!(saved(&app, WALLET_RECORD_PART)["balance"], 180);
    ok(&mut app, "wallet.withdraw 180");
    ok(&mut app, "wallet.deposit");
    tick(&mut app);
    ok(&mut app, "assert.wallet_balance 300");
}

#[test]
fn away_from_the_deposit_nothing_happens_and_the_balance_comes_back_next_time() {
    let mut app = app();
    let body = start(&mut app, 0.0, 40);
    ok(&mut app, "wallet.deposit");
    app.world_mut()
        .get_mut::<Transform>(body)
        .unwrap()
        .translation
        .x = 8.0;
    assert!(refusal(&mut app, "wallet.withdraw 10").contains("walk up to the gold deposit"));
    app.world_mut().despawn(body);
    tick(&mut app);
    app.world_mut().spawn((
        Name::new("character_ada-1_2"),
        PlayerId(0),
        character(),
        Transform::default(),
    ));
    tick(&mut app);
    ok(&mut app, "assert.wallet_balance 40");
}
