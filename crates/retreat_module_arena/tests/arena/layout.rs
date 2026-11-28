//! The converted layout and the spawn point rules.

use bevy::prelude::*;
use retreat_gameplay_api::{MAX_PLAYERS, SpawnPoint};
use retreat_module_arena::*;

fn piece<'a>(map: &'a ArenaMap, name: &str) -> &'a MapPiece {
    map.pieces
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no piece `{name}`"))
}

fn at(x: f32, y: f32, z: f32) -> SpawnPoint {
    SpawnPoint {
        feet: Vec3::new(x, y, z),
        yaw: 0.0,
    }
}

#[test]
fn source_axes_map_to_retreat_axes_and_back() {
    // Source forward +X, right +Y, up +Z, in cm.
    assert_eq!(source_cm_to_retreat(Vec3::X * 100.0), Vec3::NEG_Z);
    assert_eq!(source_cm_to_retreat(Vec3::Y * 100.0), Vec3::X);
    assert_eq!(source_cm_to_retreat(Vec3::Z * 100.0), Vec3::Y);
    for source in [
        Vec3::new(800.0, -500.0, 100.0),
        Vec3::new(-3100.0, 3000.0, -200.0),
    ] {
        let back = retreat_to_source_cm(source_cm_to_retreat(source));
        assert!(back.abs_diff_eq(source, 1.0e-2), "{source} -> {back}");
    }
}

#[test]
fn the_graybox_is_valid_and_keeps_the_planned_dimensions() {
    let map = graybox_map();
    assert_eq!(validate_map(&map), Vec::<String>::new());
    assert_eq!(map.half_extent, 30.0);
    assert_eq!(piece(&map, "floor").size(), Vec3::new(60.0, 2.0, 60.0));
    assert_eq!(piece(&map, "floor").max.y, 0.0);
    assert_eq!(piece(&map, "platform_low").max.y, 1.0);
    assert_eq!(piece(&map, "platform_high").max.y, 2.0);
    // Source x 800..1600, y -500..500 (forward and right) becomes z -16..-8, x -5..5.
    assert_eq!(piece(&map, "platform_low").min, Vec3::new(-5.0, 0.0, -16.0));
    assert_eq!(piece(&map, "platform_low").max, Vec3::new(5.0, 1.0, -8.0));
}

#[test]
fn opposite_walls_mirror_each_other() {
    let map = graybox_map();
    for (a, b, axis) in [
        ("wall_west", "wall_east", Vec3::new(-1.0, 1.0, 1.0)),
        ("wall_north", "wall_south", Vec3::new(1.0, 1.0, -1.0)),
    ] {
        let (a, b) = (piece(&map, a), piece(&map, b));
        assert_eq!(a.center() * axis, b.center());
        assert_eq!(a.size(), b.size());
        assert_eq!(a.max.y, 4.0);
    }
    assert!(piece(&map, "wall_west").center().x < 0.0);
    assert!(piece(&map, "wall_north").center().z < 0.0);
}

#[test]
fn the_stairs_climb_in_ten_centimetre_steps_to_the_low_platform() {
    let map = graybox_map();
    for step in 1..=10 {
        let stair = piece(&map, &format!("stair_{step}"));
        assert!((stair.max.y - step as f32 * 0.1).abs() < 1.0e-6);
        assert!((stair.min.z + 8.0).abs() < 1.0e-6, "ends at the platform");
    }
    assert_eq!(
        piece(&map, "stair_10").max.y,
        piece(&map, "platform_low").max.y
    );
}

#[test]
fn there_is_a_clear_spawn_point_for_every_player_facing_the_center() {
    let map = graybox_map();
    assert!(map.spawn_points.len() >= MAX_PLAYERS);
    for spawn in &map.spawn_points {
        assert_eq!(spawn_point_problem(&map, spawn), None, "{spawn:?}");
        let forward = Quat::from_rotation_y(spawn.yaw) * Vec3::NEG_Z;
        let to_center = -spawn.feet.normalize();
        assert!(forward.abs_diff_eq(to_center, 1.0e-4), "{spawn:?}");
    }
}

#[test]
fn nobody_can_spawn_inside_geometry_outside_or_in_the_air() {
    let map = graybox_map();
    let problem = |spawn| spawn_point_problem(&map, &spawn).expect("must be refused");
    // Inside every solid piece above the floor.
    for solid in map.pieces.iter().filter(|p| p.kind != PieceKind::Floor) {
        let center = solid.center();
        let spawn = at(center.x, 0.0, center.z);
        assert!(
            spawn_point_problem(&map, &spawn).is_some(),
            "{}",
            solid.name
        );
    }
    assert!(problem(at(-8.5, 0.0, 2.5)).contains("pillar_1"));
    // Only the character's radius reaches into the pillar.
    assert!(problem(at(-7.8, 0.0, 2.5)).contains("pillar_1"));
    assert!(problem(at(0.0, -0.5, 26.0)).contains("floor"));
    assert!(problem(at(29.9, 0.0, 0.0)).contains("outside"));
    assert!(problem(at(40.0, 0.0, 0.0)).contains("outside"));
    assert!(problem(at(0.0, 3.0, 26.0)).contains("does not stand"));
    assert!(problem(at(f32::NAN, 0.0, 0.0)).contains("finite"));
    // Standing on top of a platform is fine.
    assert_eq!(spawn_point_problem(&map, &at(0.0, 2.0, -20.0)), None);
}

#[test]
fn a_broken_map_lists_what_to_fix() {
    let mut map = graybox_map();
    map.spawn_points.truncate(2);
    map.spawn_points.push(map.spawn_points[0]);
    let duplicate = map.pieces[0].clone();
    map.pieces.push(duplicate);
    let problems = validate_map(&map).join("\n");
    assert!(problems.contains("`floor` is used twice"), "{problems}");
    assert!(problems.contains("at least 8"), "{problems}");
    assert!(problems.contains("overlaps an earlier one"), "{problems}");
}

#[test]
fn every_pickup_spot_is_reachable_ground_and_a_bad_one_is_named() {
    let mut map = graybox_map();
    assert!(validate_map(&map).is_empty());
    assert!(map.pickup_spots.iter().any(|spot| spot.kind == "launcher"));
    map.pickup_spots[0].feet.y = 5.0;
    let problems = validate_map(&map);
    assert!(
        problems.len() == 1 && problems[0].contains(&map.pickup_spots[0].name),
        "{problems:?}"
    );
}

#[test]
fn the_crawler_city_is_valid_with_a_safe_home_city_and_a_far_placeholder_dungeon() {
    let map = crawler_city_map();
    assert_eq!(validate_map(&map), Vec::<String>::new());
    let areas = retreat_gameplay_api::MapAreas(map.areas.clone());
    let city = areas.get("city").expect("a city area");
    assert!(city.home && city.rules.safe && !city.rules.pvp);
    let dungeon = areas.get("dungeon_placeholder").expect("a dungeon area");
    assert!(!dungeon.rules.safe);
    assert!(
        dungeon.min.x - city.max.x > 400.0,
        "the areas are far apart"
    );
    let home_points = retreat_gameplay_api::MapAreas::spawn_points_in(&map.spawn_points, city);
    assert!(
        home_points.len() >= MAX_PLAYERS,
        "everyone can start in the city"
    );
    assert!(
        !retreat_gameplay_api::MapAreas::spawn_points_in(&map.spawn_points, dungeon).is_empty()
    );
    assert!(
        map.areas
            .iter()
            .all(|area| area.contains(Vec3::new(area.min.x, 0.0, area.min.y)))
    );
}

#[test]
fn a_map_with_areas_needs_every_spawn_point_inside_one_and_a_single_home() {
    let mut map = crawler_city_map();
    map.spawn_points.push(at(200.0, 0.0, 0.0));
    let problems = validate_map(&map);
    assert!(
        problems.iter().any(|p| p.contains("is in no area")),
        "{problems:?}"
    );
    let mut map = crawler_city_map();
    map.areas[1].home = true;
    let problems = validate_map(&map);
    assert!(
        problems.iter().any(|p| p.contains("2 home areas")),
        "{problems:?}"
    );
    let mut map = crawler_city_map();
    map.areas[0].rules.pvp = true;
    let problems = validate_map(&map);
    assert!(
        problems.iter().any(|p| p.contains("cannot allow PvP")),
        "{problems:?}"
    );
    let mut map = crawler_city_map();
    map.areas[1].min = Vec2::new(30.0, -10.0);
    let problems = validate_map(&map);
    assert!(
        problems.iter().any(|p| p.contains("overlap")),
        "{problems:?}"
    );
}
