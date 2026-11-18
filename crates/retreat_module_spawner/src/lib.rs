//! Spawner module: `spawner.rain <count>` drops `count` boxes from above the scene.
//!
//! The first Retreat module, and the template for the next ones:
//! - a Bevy plugin ([`SpawnerModulePlugin`]) that a game adds;
//! - only `bevy` and `retreat_engine_api` as dependencies (no engine internals, no physics
//!   library: bodies are `retreat_engine_api::PhysicsBody` descriptions);
//! - every user-visible action is a command, registered through `AppCommandExt`;
//! - randomness only from `SimulationRng`, so the same seed drops the same boxes everywhere;
//! - a manifest in `[package.metadata.retreat]` of its Cargo.toml.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, CommandArgs, CommandOutcome, CommandResult, PhysicsBody, SimulationRng,
};

/// Most boxes one `spawner.rain` may drop (keeps a typo like `spawner.rain 10000` harmless).
pub const MAX_RAIN_COUNT: u32 = 64;

/// Boxes appear in a square of this half-width around the origin, on x and z.
const RAIN_SPREAD: f32 = 2.5;
/// Height of the lowest box.
const RAIN_BASE_HEIGHT: f32 = 4.0;
/// Each next box starts this much higher, so boxes of one call never start inside each other.
const RAIN_HEIGHT_STEP: f32 = 0.6;
const MIN_BOX_SIZE: f32 = 0.25;
const MAX_BOX_SIZE: f32 = 0.5;
const RAIN_BOX_COLOR: Color = Color::srgb(0.25, 0.55, 0.95);

/// Adds the `spawner.rain` command.
pub struct SpawnerModulePlugin;

impl Plugin for SpawnerModulePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RainBoxCount>().register_command(
            "spawner.rain",
            "spawner.rain <count>",
            rain_command,
        );
    }
}

/// Marks a box dropped by `spawner.rain`.
#[derive(Component)]
pub struct RainBox;

/// How many boxes `spawner.rain` has made; names continue from it (`rain_1`, `rain_2`, ...).
#[derive(Resource, Default)]
struct RainBoxCount(u32);

/// `spawner.rain <count>`: drops `count` boxes (1 to [`MAX_RAIN_COUNT`]) named `rain_<n>`, at
/// `SimulationRng` positions and sizes, stacked in height so none overlap at the start.
fn rain_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let count: u32 = args.parse(0, "count")?;
    if !(1..=MAX_RAIN_COUNT).contains(&count) {
        return Err(args.failure(format!(
            "<count> must be from 1 to {MAX_RAIN_COUNT}, got {count}"
        )));
    }
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(RAIN_BOX_COLOR);
    for index in 0..count {
        let name = next_free_rain_name(world);
        let mut rng = world.resource_mut::<SimulationRng>();
        let x = rng.0.range_f32(-RAIN_SPREAD, RAIN_SPREAD);
        let z = rng.0.range_f32(-RAIN_SPREAD, RAIN_SPREAD);
        let size = rng.0.range_f32(MIN_BOX_SIZE, MAX_BOX_SIZE);
        let y = RAIN_BASE_HEIGHT + index as f32 * RAIN_HEIGHT_STEP;
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::from_length(size));
        world.spawn((
            Name::new(name),
            RainBox,
            PhysicsBody::dynamic_cube(size),
            Transform::from_xyz(x, y, z),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
        ));
    }
    Ok(CommandOutcome::Continue)
}

/// `rain_<n>`: the first number whose name nobody uses yet (another module or a script may
/// have named something `rain_3`; names must stay unique).
fn next_free_rain_name(world: &mut World) -> String {
    let mut names = world.query::<&Name>();
    let taken: Vec<String> = names
        .iter(world)
        .map(|name| name.as_str().to_string())
        .collect();
    let mut count = world.resource_mut::<RainBoxCount>();
    loop {
        count.0 += 1;
        let candidate = format!("rain_{}", count.0);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
}
