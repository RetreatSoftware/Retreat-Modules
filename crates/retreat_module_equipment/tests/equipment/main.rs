//! Equipment with only Bevy and the contract crates: the slot and level rules, two-handed weapons,
//! the test level and saving. A hand-made bag stands in for the bag module.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, RecordStore, RemoveNetEntity, SimulationRng, run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_equipment::*;

fn catalog() -> GridItemCatalog {
    let base = |id: &str, size: UVec2, slots: Vec<EquipSlot>, two_handed: bool| ItemBase {
        id: id.to_string(),
        name: id.to_string(),
        size,
        slots,
        two_handed,
        max_stack: 1,
        rollable: true,
        affix_pool: Vec::new(),
        icon: None,
        color: [0.5; 3],
    };
    GridItemCatalog {
        bases: vec![
            base("sword", UVec2::new(1, 3), vec![EquipSlot::MainHand], false),
            base(
                "greatsword",
                UVec2::new(1, 4),
                vec![EquipSlot::MainHand],
                true,
            ),
            base("shield", UVec2::new(2, 2), vec![EquipSlot::OffHand], false),
            base("helmet", UVec2::new(2, 2), vec![EquipSlot::Head], false),
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
        .add_plugins(EquipmentModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app
}

fn bag_id() -> ContainerId {
    container_id(ContainerKind::Bag, &character().id)
}

/// A playing body and a hand-made 6 x 6 bag holding `bases` (one item each, at free spots).
fn start(app: &mut App, bases: &[&str]) -> (Entity, Vec<ItemUid>) {
    let body = app
        .world_mut()
        .spawn((
            Name::new("character_ada-1_1"),
            PlayerId(0),
            character(),
            Transform::default(),
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
    app.world_mut().run_schedule(FixedUpdate);
    let catalog = catalog();
    let unique = bases
        .iter()
        .map(|base| GridItem {
            uid: ItemUid(0),
            base: base.to_string(),
            rarity: "common".to_string(),
            affixes: Vec::new(),
            level_requirement: if *base == "helmet" { 3 } else { 1 },
            quantity: 1,
            size: catalog.base(base).unwrap().size,
        })
        .collect();
    let uids = add_to_container(
        app.world_mut(),
        PlayerId(0),
        &bag_id(),
        NewItems {
            unique,
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

fn refusal(app: &mut App, line: &str) -> String {
    run(app, line)
        .err()
        .unwrap_or_else(|| panic!("`{line}` must be refused"))
        .to_string()
}

fn slot(slot: EquipSlot) -> u32 {
    slot.index()
}

#[test]
fn items_go_only_in_their_slot_and_only_at_their_level() {
    let mut app = app();
    let (body, uids) = start(&mut app, &["helmet", "sword"]);
    let (helmet, sword) = (uids[0].0, uids[1].0);
    assert!(
        refusal(
            &mut app,
            &format!("equipment.put {sword} {} 0 0", slot(EquipSlot::Head))
        )
        .contains("cannot be worn in the head slot")
    );
    let text = refusal(
        &mut app,
        &format!("equipment.put {helmet} {} 0 0", slot(EquipSlot::Head)),
    );
    assert!(text.contains("needs level 3 (you are level 1)"), "{text}");
    assert!(
        app.world()
            .get::<ItemNotice>(body)
            .unwrap()
            .text
            .contains("needs level 3")
    );
    ok(&mut app, "equipment.test_level_up");
    ok(&mut app, "equipment.test_level_up");
    assert_eq!(
        app.world().get::<CharacterLevel>(body),
        Some(&CharacterLevel(3))
    );
    ok(
        &mut app,
        &format!("equipment.put {helmet} {} 0 0", slot(EquipSlot::Head)),
    );
    ok(&mut app, "assert.equipped head helmet");
    ok(&mut app, "assert.equipped main_hand nothing");
    // Taking it off is a move back into the bag.
    move_item(
        app.world_mut(),
        PlayerId(0),
        uids[0],
        &bag_id(),
        4,
        4,
        false,
    )
    .unwrap();
    ok(&mut app, "assert.equipped head nothing");
}

#[test]
fn a_two_handed_weapon_takes_both_hands() {
    let mut app = app();
    let (_, uids) = start(&mut app, &["greatsword", "shield", "sword"]);
    let (great, shield, sword) = (uids[0].0, uids[1].0, uids[2].0);
    ok(
        &mut app,
        &format!("equipment.put {shield} {} 0 0", slot(EquipSlot::OffHand)),
    );
    assert!(
        refusal(
            &mut app,
            &format!("equipment.put {great} {} 0 0", slot(EquipSlot::MainHand))
        )
        .contains("needs both hands")
    );
    ok(
        &mut app,
        &format!("equipment.put {sword} {} 0 0", slot(EquipSlot::MainHand)),
    );
    assert!(
        refusal(
            &mut app,
            &format!("equipment.put {great} {} 0 0", slot(EquipSlot::MainHand))
        )
        .contains("take it off first")
    );
}

#[test]
fn worn_items_are_saved_and_come_back() {
    let mut app = app();
    let (body, uids) = start(&mut app, &["sword"]);
    ok(
        &mut app,
        &format!(
            "equipment.put {} {} 0 0",
            uids[0].0,
            slot(EquipSlot::MainHand)
        ),
    );
    app.world_mut().despawn(body);
    app.world_mut().run_schedule(FixedUpdate);
    let removed: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<RemoveNetEntity>>()
        .iter(app.world())
        .collect();
    for entity in removed {
        app.world_mut().despawn(entity);
    }
    ok_absent(&mut app);
    app.world_mut().spawn((
        Name::new("character_ada-1_2"),
        PlayerId(0),
        character(),
        Transform::default(),
    ));
    app.world_mut().run_schedule(FixedUpdate);
    ok(&mut app, "assert.equipped main_hand sword");
}

fn ok_absent(app: &mut App) {
    let id = container_id(ContainerKind::Equipment, &character().id);
    assert!(container_entity(app.world_mut(), &id).is_none());
}
