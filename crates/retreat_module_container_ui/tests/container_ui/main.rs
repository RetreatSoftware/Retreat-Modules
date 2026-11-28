//! The container UI with only Bevy and the contract crates: what a window shows (items, tooltips,
//! footers, slots), opening and closing, a chest opening the windows by itself, and only this
//! machine's character's containers. Drawing and dragging are the engine's, checked with pictures
//! in the crawler testbed (Dungeon-Crawler).

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CursorCapture, UiGridShape, UiGrids, UiHud, UiPanels, run_command_line,
};
use retreat_gameplay_api::*;
use retreat_module_container_ui::*;

fn catalog() -> GridItemCatalog {
    GridItemCatalog {
        bases: vec![
            ItemBase {
                id: "greatsword".to_string(),
                name: "Greatsword".to_string(),
                size: UVec2::new(1, 4),
                slots: vec![EquipSlot::MainHand],
                two_handed: true,
                max_stack: 1,
                rollable: true,
                affix_pool: vec!["strength".to_string()],
                icon: Some("icons/greatsword.png".to_string()),
                color: [0.5; 3],
            },
            ItemBase {
                id: "gold".to_string(),
                name: "Gold".to_string(),
                size: UVec2::ONE,
                slots: Vec::new(),
                two_handed: false,
                max_stack: 500,
                rollable: false,
                affix_pool: Vec::new(),
                icon: None,
                color: [0.9, 0.8, 0.2],
            },
        ],
        rarities: vec![ItemRarity {
            id: "rare".to_string(),
            name: "Rare".to_string(),
            weight: 1,
            affixes: (1, 1),
            color: [1.0, 0.9, 0.2],
        }],
        affixes: vec![ItemAffix {
            id: "strength".to_string(),
            text: "+{} strength".to_string(),
            min: 1,
            max: 3,
            per_level: 1,
        }],
    }
}

fn container(kind: ContainerKind, character: &str, layout: ContainerLayout) -> ItemContainer {
    let character = CharacterId(character.to_string());
    ItemContainer {
        id: container_id(kind, &character),
        kind,
        title: kind.word().to_string(),
        layout,
        character,
        only_in_safe_area: false,
        drop_command: format!("{}.put", kind.word()),
        record_owner: "character/x".to_string(),
        record_part: "x".to_string(),
    }
}

fn bag(character: &str) -> ItemContainer {
    container(
        ContainerKind::Bag,
        character,
        ContainerLayout::Grid {
            columns: 6,
            rows: 4,
            visible_rows: 0,
        },
    )
}

fn placed(
    uid: u64,
    base: &str,
    quantity: u32,
    level: u32,
    container: &ItemContainer,
) -> PlacedItem {
    PlacedItem {
        item: GridItem {
            uid: ItemUid(uid),
            base: base.to_string(),
            rarity: "rare".to_string(),
            affixes: if base == "gold" {
                Vec::new()
            } else {
                vec![RolledAffix {
                    affix: "strength".to_string(),
                    value: 4,
                }]
            },
            level_requirement: level,
            quantity,
            size: catalog().base(base).unwrap().size,
        },
        place: ItemPlace {
            container: container.id.clone(),
            x: 1,
            y: 0,
            rotated: true,
        },
    }
}

#[test]
fn a_window_shows_items_with_icons_counts_and_tooltips() {
    let bag = bag("ada-1");
    let items = vec![
        placed(7, "greatsword", 1, 5, &bag),
        placed(8, "gold", 120, 0, &bag),
    ];
    let view = container_view(
        &catalog(),
        &ContainerUiConfig::default(),
        &bag,
        &items,
        3,
        Some(40),
    );
    assert_eq!(view.drop_command, "bag.put");
    assert_eq!(view.footer, "Gold carried: 120   Safe balance: 40");
    let sword = view.item("7").unwrap();
    assert!(sword.rotated);
    assert_eq!(sword.image.as_deref(), Some("icons/greatsword.png"));
    assert_eq!(sword.label, "", "an item with an icon needs no name");
    let lines: Vec<&str> = sword
        .tooltip
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(
        lines,
        [
            "Greatsword",
            "Rare - Main hand",
            "+4 strength",
            "Two-handed",
            "Requires level 5"
        ]
    );
    assert_ne!(
        sword.tooltip[4].color,
        container_view(
            &catalog(),
            &ContainerUiConfig::default(),
            &bag,
            &items,
            5,
            None
        )
        .item("7")
        .unwrap()
        .tooltip[4]
            .color,
        "the requirement is red only while the level is below it"
    );
    let gold = view.item("8").unwrap();
    assert_eq!(gold.label, "120");
    assert!(gold.tooltip.iter().any(|line| line.text == "120 of 500"));
}

#[test]
fn the_equipment_window_has_one_slot_per_equipment_slot() {
    let equipment = container(
        ContainerKind::Equipment,
        "ada-1",
        ContainerLayout::Equipment,
    );
    let view = container_view(
        &catalog(),
        &ContainerUiConfig::default(),
        &equipment,
        &[],
        2,
        None,
    );
    let UiGridShape::Slots(slots) = &view.shape else {
        panic!("equipment uses slots");
    };
    assert_eq!(slots.len(), EquipSlot::ALL.len());
    assert_eq!(slots[EquipSlot::OffHand.index() as usize].label, "Off hand");
    assert!(view.validate().is_ok());
    assert_eq!(view.footer, "Level 2");
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<UiPanels>()
        .init_resource::<UiHud>()
        .init_resource::<UiGrids>()
        .init_resource::<CursorCapture>()
        .insert_resource(catalog())
        .add_plugins(ContainerUiModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut().spawn((
        Name::new("character_ada-1_1"),
        PlayerId(0),
        PlayingCharacter {
            id: CharacterId("ada-1".to_string()),
            name: "Ada".to_string(),
            account: AccountId("ada".to_string()),
        },
    ));
    app
}

fn shown(app: &App) -> Vec<String> {
    app.world()
        .resource::<UiGrids>()
        .views()
        .map(|(id, _)| id.to_string())
        .collect()
}

#[test]
fn windows_open_on_request_show_only_this_characters_containers_and_open_for_a_chest() {
    let mut app = app();
    app.world_mut()
        .spawn((Name::new("container_bag_ada-1"), bag("ada-1")));
    app.world_mut()
        .spawn((Name::new("container_bag_bob-1"), bag("bob-1")));
    app.update();
    assert!(shown(&app).is_empty(), "closed until asked");
    assert!(
        app.world()
            .resource::<UiPanels>()
            .is_visible("container_ui")
    );
    app.world_mut().resource_mut::<CursorCapture>().wanted = true;
    run_command_line(app.world_mut(), "container_ui.toggle").unwrap();
    app.update();
    assert_eq!(shown(&app), ["bag"]);
    assert!(
        !app.world().resource::<CursorCapture>().wanted,
        "opening frees the pointer"
    );
    run_command_line(app.world_mut(), "container_ui.close").unwrap();
    app.update();
    assert!(shown(&app).is_empty());
    app.world_mut().spawn((
        Name::new("container_chest_ada-1"),
        container(
            ContainerKind::Chest,
            "ada-1",
            ContainerLayout::Grid {
                columns: 10,
                rows: 8,
                visible_rows: 6,
            },
        ),
    ));
    app.update();
    assert_eq!(shown(&app), ["bag", "chest"], "a chest opens the windows");
}
