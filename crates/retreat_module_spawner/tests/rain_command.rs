//! `spawner.rain` with only Bevy and the contract crate: what it spawns, that it follows the
//! seed, and its errors. The engine-side picture and physics are checked by the engine's
//! `scripts/verify/module_spawner.cmds`.

use bevy::prelude::*;
use retreat_engine_api::{
    CommandRegistry, CommandResult, PhysicsBody, SimulationRng, run_command_line,
};
use retreat_module_spawner::{MAX_RAIN_COUNT, RainBox, SpawnerModulePlugin};

/// An app with what the module needs and nothing from the engine: the command registry, the
/// simulation RNG and mesh/material asset stores.
fn module_app(seed: u64) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_resource::<CommandRegistry>()
        .insert_resource(SimulationRng::from_seed(seed))
        .add_plugins(SpawnerModulePlugin);
    app
}

fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

/// `(name, position)` of every rain box, sorted by name.
fn rain_boxes(app: &mut App) -> Vec<(String, Vec3)> {
    let world = app.world_mut();
    let mut query =
        world.query_filtered::<(&Name, &Transform), (With<RainBox>, With<PhysicsBody>)>();
    let mut boxes: Vec<(String, Vec3)> = query
        .iter(world)
        .map(|(name, transform)| (name.to_string(), transform.translation))
        .collect();
    boxes.sort_by(|(left, _), (right, _)| left.cmp(right));
    boxes
}

#[test]
fn rain_drops_named_physics_boxes_stacked_in_height() {
    let mut app = module_app(0);
    run(&mut app, "spawner.rain 3").expect("valid command");
    let boxes = rain_boxes(&mut app);
    let names: Vec<&str> = boxes.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["rain_1", "rain_2", "rain_3"]);
    assert!(
        boxes[0].1.y < boxes[1].1.y && boxes[1].1.y < boxes[2].1.y,
        "each box starts higher than the previous one: {boxes:?}"
    );
}

#[test]
fn a_second_rain_continues_the_names() {
    let mut app = module_app(0);
    run(&mut app, "spawner.rain 2").expect("valid command");
    run(&mut app, "spawner.rain 1").expect("valid command");
    let names: Vec<String> = rain_boxes(&mut app)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["rain_1", "rain_2", "rain_3"]);
}

#[test]
fn the_same_seed_drops_the_same_boxes_and_another_seed_does_not() {
    let positions = |seed| {
        let mut app = module_app(seed);
        run(&mut app, "spawner.rain 4").expect("valid command");
        rain_boxes(&mut app)
    };
    assert_eq!(positions(7), positions(7));
    assert_ne!(positions(7), positions(8));
}

#[test]
fn counts_outside_the_allowed_range_are_errors() {
    let mut app = module_app(0);
    for count in [0, MAX_RAIN_COUNT + 1] {
        let error = run(&mut app, &format!("spawner.rain {count}"))
            .expect_err("out of range")
            .to_string();
        assert!(error.contains("<count> must be from 1 to"), "{error}");
    }
    assert!(
        rain_boxes(&mut app).is_empty(),
        "a failed command spawns nothing"
    );
}
