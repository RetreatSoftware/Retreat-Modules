//! A minimal app that runs fixed ticks one at a time.

use bevy::prelude::*;
use retreat_engine_api::{CommandRegistry, CommandResult, run_command_line};
use retreat_gameplay_api::{PlayerId, SpawnPoint, SpawnPoints};
use retreat_module_health::{HealthConfig, HealthModulePlugin};

/// Ticks of the default 64 Hz clock in the default delays.
pub const PROTECTION_TICKS: u32 = 64;
pub const RESPAWN_TICKS: u32 = 192;

pub fn app_with(config: HealthConfig) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .insert_resource(config)
        .add_plugins(HealthModulePlugin);
    app.world_mut().resource_mut::<SpawnPoints>().0 = [(0.0, 20.0), (0.0, -20.0), (20.0, 0.0)]
        .map(|(x, z)| SpawnPoint {
            feet: Vec3::new(x, 0.0, z),
            yaw: 0.0,
        })
        .to_vec();
    app
}

pub fn app() -> App {
    app_with(HealthConfig::default())
}

pub fn run(app: &mut App, line: &str) -> CommandResult {
    run_command_line(app.world_mut(), line)
}

pub fn ok(app: &mut App, line: &str) {
    run(app, line).unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

pub fn ticks(app: &mut App, count: u32) {
    for _ in 0..count {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

/// An app with dummies `1..=count` whose spawn protection is over.
pub fn app_with_ready_players(count: u32) -> App {
    let mut app = app();
    for id in 1..=count {
        ok(&mut app, &format!("health.add_dummy {id}"));
    }
    ticks(&mut app, PROTECTION_TICKS + 1);
    app
}

pub fn position(app: &mut App, id: u32) -> Vec3 {
    let world = app.world_mut();
    let mut query = world.query::<(&PlayerId, &Transform)>();
    query
        .iter(world)
        .find(|(player, _)| player.0 == id)
        .map(|(_, transform)| transform.translation)
        .expect("player with a position")
}

/// Takes every message of a type written so far.
pub fn drain<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut()
        .resource_mut::<Messages<M>>()
        .drain()
        .collect()
}
