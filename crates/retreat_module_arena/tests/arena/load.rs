//! `arena.load`.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, PhysicsBody, PhysicsBodyKind, PhysicsShape, run_command_line,
};
use retreat_gameplay_api::SpawnPoints;
use retreat_module_arena::*;

fn module_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_resource::<CommandRegistry>()
        .add_plugins(ArenaModulePlugin);
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

fn piece_count(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&ArenaPiece>().iter(world).count()
}

#[test]
fn load_builds_every_piece_as_a_fixed_box_where_the_map_says() {
    let mut app = module_app();
    assert!(app.world().resource::<SpawnPoints>().0.is_empty());
    run(&mut app, "arena.load graybox").expect("valid command");
    let map = graybox_map();
    let world = app.world_mut();
    let mut query = world.query::<(&Name, &Transform, &PhysicsBody)>();
    let built: Vec<(String, Vec3, PhysicsBody)> = query
        .iter(world)
        .map(|(name, transform, body)| (name.to_string(), transform.translation, body.clone()))
        .collect();
    assert_eq!(built.len(), map.pieces.len());
    for piece in &map.pieces {
        let name = piece_entity_name(&piece.name);
        let (_, position, body) = built
            .iter()
            .find(|(built_name, _, _)| *built_name == name)
            .unwrap_or_else(|| panic!("no entity {name}"));
        assert_eq!(*position, piece.center());
        assert_eq!(body.kind, PhysicsBodyKind::Fixed);
        assert_eq!(
            body.shape,
            PhysicsShape::Cuboid {
                half_extents: piece.size() / 2.0
            }
        );
    }
    assert_eq!(app.world().resource::<SpawnPoints>().0, map.spawn_points);
    assert_eq!(app.world().resource::<LoadedArena>().0, "graybox");
}

#[test]
fn an_unknown_map_names_the_known_ones_and_builds_nothing() {
    let mut app = module_app();
    let error = run(&mut app, "arena.load castle")
        .expect_err("unknown")
        .to_string();
    assert!(error.contains("known maps: graybox"), "{error}");
    assert_eq!(piece_count(&mut app), 0);
}

#[test]
fn a_second_load_is_refused() {
    let mut app = module_app();
    run(&mut app, "arena.load graybox").expect("first load");
    let before = piece_count(&mut app);
    let error = run(&mut app, "arena.load graybox")
        .expect_err("second load")
        .to_string();
    assert!(error.contains("already loaded"), "{error}");
    assert_eq!(piece_count(&mut app), before);
}

#[test]
fn a_taken_name_is_refused_before_anything_is_built() {
    let mut app = module_app();
    app.world_mut().spawn(Name::new("arena/pillar_1"));
    let error = run(&mut app, "arena.load graybox")
        .expect_err("name taken")
        .to_string();
    assert!(error.contains("arena/pillar_1"), "{error}");
    assert_eq!(piece_count(&mut app), 0);
}
