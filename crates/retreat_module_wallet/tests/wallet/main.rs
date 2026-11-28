//! The wallet with only Bevy and the contract crates: deposits and withdrawals at a deposit spot,
//! one atomic save of bag and balance, refusals, the balance coming back, and one balance for all
//! of an account's characters in every world (two apps on one shared store are two world servers),
//! also when two worlds deposit at the same moment. A hand-made bag stands in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, RecordBackend, RecordChange, RecordCommitError,
    RecordExpectation, RecordKey, RecordStore, RemoveNetEntity, SharedMemoryRecordBackend,
    SimulationRng, encode_record, run_command_line,
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

fn app() -> App {
    app_on(RecordStore::in_memory())
}

/// A world server whose records are `store` (apps on one shared store are the worlds of a game).
fn app_on(store: RecordStore) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(store)
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
    start_as(app, character(), x, gold)
}

/// `playing` (player 0 of its world) at `x` with a hand-made 2 x 2 bag holding `gold`.
fn start_as(app: &mut App, playing: PlayingCharacter, x: f32, gold: u32) -> Entity {
    let bag = container_id(ContainerKind::Bag, &playing.id);
    let body = app
        .world_mut()
        .spawn((
            Name::new(format!("character_{}_1", playing.id.0)),
            PlayerId(0),
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
                columns: 2,
                rows: 2,
                visible_rows: 0,
            },
            character: playing.id.clone(),
            only_in_safe_area: false,
            drop_command: "test_bag.put".to_string(),
            record_owner: character_record_owner(&playing.id),
            record_part: "test_bag".to_string(),
        },
        PlayerId(0),
        None,
    );
    if gold > 0 {
        add_to_container(
            app.world_mut(),
            PlayerId(0),
            &bag,
            NewItems {
                unique: Vec::new(),
                stack: Some(("gold".to_string(), gold)),
            },
            ExtraRecords::default(),
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

fn carried(app: &mut App) -> u32 {
    container_items(app.world_mut(), &bag_id())
        .iter()
        .map(|placed| placed.item.quantity)
        .sum()
}

fn saved(app: &App, owner: &str, part: &str) -> retreat_engine_api::serde_json::Value {
    app.world()
        .resource::<RecordStore>()
        .read(&RecordKey::new(owner, part))
        .unwrap()
        .expect("saved")
        .data
}

fn carried_by(app: &mut App, playing: &PlayingCharacter) -> u32 {
    container_items(
        app.world_mut(),
        &container_id(ContainerKind::Bag, &playing.id),
    )
    .iter()
    .map(|placed| placed.item.quantity)
    .sum()
}

fn ticks(app: &mut App, count: usize) {
    for _ in 0..count {
        tick(app);
    }
}

/// Enough ticks for a shown balance to be read again (the default half second at 64 Hz is 32).
const REFRESH_TICKS: usize = 40;

/// The shared records of every world, where one other world's write lands at the moment the next
/// commit is checked: two worlds depositing at the same moment.
struct RacingBackend {
    records: SharedMemoryRecordBackend,
    other_world_writes: std::sync::Arc<std::sync::Mutex<Option<(RecordKey, u64)>>>,
}

impl RecordBackend for RacingBackend {
    fn read(&self, key: &RecordKey) -> Result<Option<String>, String> {
        self.records.read(key)
    }

    fn parts(&self, owner: &str) -> Result<Vec<String>, String> {
        self.records.parts(owner)
    }

    fn commit(
        &mut self,
        expected: &[RecordExpectation],
        changes: &[RecordChange],
    ) -> Result<(), RecordCommitError> {
        if let Some((key, balance)) = self.other_world_writes.lock().unwrap().take() {
            let text = encode_record(
                &key,
                1,
                &retreat_engine_api::serde_json::json!({ "balance": balance }),
            );
            self.records
                .commit(&[], &[RecordChange::Put { key, text }])?;
        }
        self.records.commit(expected, changes)
    }

    fn location(&self) -> String {
        "racing memory".to_string()
    }
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
    assert_eq!(
        saved(&app, "account/ada", WALLET_RECORD_PART)["balance"],
        150
    );
    assert_eq!(
        saved(&app, "character/ada-1", "test_bag")["items"]
            .as_array()
            .unwrap()
            .len(),
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
    assert_eq!(
        saved(&app, "account/ada", WALLET_RECORD_PART)["balance"],
        180
    );
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

#[test]
fn every_character_of_the_account_in_every_world_shares_one_balance() {
    let shared = SharedMemoryRecordBackend::default();
    let mut one = app_on(RecordStore::new(Box::new(shared.clone())));
    let mut two = app_on(RecordStore::new(Box::new(shared.clone())));
    let ada = character_of_ada(1, "Ada");
    let ann = character_of_ada(2, "Ann");
    start_as(&mut one, ada.clone(), 0.0, 150);
    start_as(&mut two, ann.clone(), 0.0, 60);
    ok(&mut one, "wallet.deposit");
    ok(&mut two, "wallet.deposit");
    ok(&mut two, "assert.wallet_balance 210");
    // World one shows world two's deposit after a moment.
    ticks(&mut one, REFRESH_TICKS);
    ok(&mut one, "assert.wallet_balance 210");
    // Ann takes out more than she put in.
    ok(&mut two, "wallet.withdraw 100");
    assert_eq!(carried_by(&mut two, &ann), 100);
    ticks(&mut one, REFRESH_TICKS);
    ok(&mut one, "assert.wallet_balance 110");
    let store = RecordStore::new(Box::new(shared));
    let balance = store
        .read(&RecordKey::new("account/ada", WALLET_RECORD_PART))
        .unwrap()
        .unwrap();
    assert_eq!(balance.data["balance"], 110);
    assert!(
        store
            .read(&RecordKey::new("character/ada-1", WALLET_RECORD_PART))
            .unwrap()
            .is_none(),
        "no balance per character"
    );
}

#[test]
fn two_worlds_depositing_at_the_same_moment_never_lose_or_double_gold() {
    let shared = SharedMemoryRecordBackend::default();
    let other_world_writes = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut one = app_on(RecordStore::new(Box::new(RacingBackend {
        records: shared.clone(),
        other_world_writes: other_world_writes.clone(),
    })));
    let ada = character_of_ada(1, "Ada");
    start_as(&mut one, ada.clone(), 0.0, 150);
    // World two deposits 60 between world one reading the balance (0) and writing it.
    *other_world_writes.lock().unwrap() = Some((wallet_key(&ada.account), 60));
    let why = refusal(&mut one, "wallet.deposit");
    assert!(why.contains("another of your characters"), "{why}");
    assert_eq!(carried_by(&mut one, &ada), 150, "nothing moved");
    ok(&mut one, "assert.wallet_balance 60");
    // Trying again adds to what is saved now.
    ok(&mut one, "wallet.deposit");
    ok(&mut one, "assert.wallet_balance 210");
    assert_eq!(carried_by(&mut one, &ada), 0);
    let store = RecordStore::new(Box::new(shared));
    let balance = store.read(&wallet_key(&ada.account)).unwrap().unwrap();
    assert_eq!(balance.data["balance"], 210);
    let bag = store
        .read(&RecordKey::new("character/ada-1", "test_bag"))
        .unwrap()
        .unwrap();
    assert!(bag.data["items"].as_array().unwrap().is_empty());
}
