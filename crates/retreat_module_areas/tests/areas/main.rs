//! The areas module with only Bevy and the contract crates. With the real engine it is checked in
//! the crawler testbed (Dungeon-Crawler).

use bevy::prelude::*;
use retreat_engine_api::{CommandRegistry, CommandResult, PhysicsTeleport, run_command_line};
use retreat_gameplay_api::*;
use retreat_module_areas::*;

fn rules(safe: bool) -> AreaRules {
    AreaRules {
        safe,
        pvp: !safe,
        tier: u32::from(!safe),
        max_entry_level: None,
    }
}

fn areas() -> MapAreas {
    MapAreas(vec![
        Area {
            id: AreaId("city".to_string()),
            name: "City".to_string(),
            rules: rules(true),
            min: Vec2::splat(-10.0),
            max: Vec2::splat(10.0),
            home: true,
        },
        Area {
            id: AreaId("cave".to_string()),
            name: "Cave".to_string(),
            rules: rules(false),
            min: Vec2::new(90.0, -10.0),
            max: Vec2::new(110.0, 10.0),
            home: false,
        },
    ])
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(AreasModulePlugin);
    app.insert_resource(areas());
    app.world_mut().resource_mut::<SpawnPoints>().0 = vec![
        SpawnPoint {
            feet: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
        },
        SpawnPoint {
            feet: Vec3::new(100.0, 0.0, 0.0),
            yaw: 1.0,
        },
    ];
    app
}

fn character(app: &mut App, id: u32, feet: Vec3) -> Entity {
    let size = *app.world().resource::<CharacterSize>();
    app.world_mut()
        .spawn((
            Name::new(player_entity_name(PlayerId(id))),
            PlayerId(id),
            LookAngles::default(),
            Transform::from_translation(size.center(feet)),
        ))
        .id()
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

#[test]
fn the_area_is_where_the_character_stands_else_the_last_one_else_home() {
    let areas = areas();
    let cave = AreaId("cave".to_string());
    assert_eq!(
        area_for(&areas, Vec3::new(1.0, 0.0, 1.0), None)
            .unwrap()
            .id
            .0,
        "city"
    );
    assert_eq!(
        area_for(&areas, Vec3::new(95.0, 3.0, 0.0), None)
            .unwrap()
            .id
            .0,
        "cave"
    );
    assert_eq!(
        area_for(&areas, Vec3::new(50.0, -20.0, 0.0), Some(&cave))
            .unwrap()
            .id
            .0,
        "cave",
        "between areas a character keeps its area"
    );
    assert_eq!(
        area_for(&areas, Vec3::new(50.0, 0.0, 0.0), None)
            .unwrap()
            .id
            .0,
        "city"
    );
    assert!(area_for(&MapAreas::default(), Vec3::ZERO, None).is_none());
}

#[test]
fn every_character_gets_exactly_one_area() {
    let mut app = app();
    let in_city = character(&mut app, 1, Vec3::new(2.0, 0.0, 2.0));
    let in_cave = character(&mut app, 2, Vec3::new(100.0, 0.0, 3.0));
    tick(&mut app);
    tick(&mut app);
    assert_eq!(app.world().get::<InArea>(in_city).unwrap().0.0, "city");
    assert_eq!(app.world().get::<InArea>(in_cave).unwrap().0.0, "cave");
    run(&mut app, "assert.area 2 cave").unwrap();
    let error = run(&mut app, "assert.area 1 cave").unwrap_err().to_string();
    assert!(
        error.contains("is in area `city`, expected `cave`"),
        "{error}"
    );
}

#[test]
fn moving_puts_the_character_at_a_spawn_point_of_the_other_area() {
    let mut app = app();
    let body = character(&mut app, 1, Vec3::new(2.0, 0.0, 2.0));
    tick(&mut app);
    run(&mut app, "areas.move 1 cave").unwrap();
    let size = *app.world().resource::<CharacterSize>();
    let teleport = app
        .world()
        .get::<PhysicsTeleport>(body)
        .expect("teleported");
    assert_eq!(teleport.0, size.center(Vec3::new(100.0, 0.0, 0.0)));
    assert_eq!(app.world().get::<InArea>(body).unwrap().0.0, "cave");
    assert_eq!(app.world().get::<LookAngles>(body).unwrap().yaw, 1.0);
    // The physics step has not moved the body yet; the pending teleport already counts.
    tick(&mut app);
    run(&mut app, "assert.area 1 cave").unwrap();
    let error = run(&mut app, "areas.move 1 moon").unwrap_err().to_string();
    assert!(
        error.contains("no area `moon`; areas: city, cave"),
        "{error}"
    );
    let error = run(&mut app, "areas.move 9 city").unwrap_err().to_string();
    assert!(error.contains("player 9 has no character"), "{error}");
    run(&mut app, "areas.list").unwrap();
}
