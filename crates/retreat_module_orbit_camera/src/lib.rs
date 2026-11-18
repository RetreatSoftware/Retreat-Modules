//! Orbit camera module: rotates a `GameCamera` around its target at a fixed radius and height.
//!
//! Built only on the contract crate: the camera is the contract's `GameCamera` (never Bevy's
//! `Transform`), and the keys are input actions read as held (`InputActions::is_held`), so the
//! module has no platform code and no input library.
//!
//! - `orbit.start <camera> <radius> <height>`: begins orbiting the named camera around its
//!   current target, at angle 0 (the camera sits on +z of the target).
//! - `orbit.stop`: stops orbiting (the camera stays where it is).
//! - `orbit.set_angle <degrees>`: jumps to an angle (for scripts and tests).
//! - Actions `orbit_left` (ArrowLeft) and `orbit_right` (ArrowRight): while held, the angle
//!   changes by [`ORBIT_DEGREES_PER_SECOND`] of fixed time.
//!
//! Angle 0 puts the camera at `target + (0, height, radius)`; the angle grows toward +x.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, CommandArgs, CommandError, CommandOutcome, CommandResult,
    GameCamera, HELD_ONLY, InputAction, InputActionSystems, InputActions, InputKey,
};

/// How fast a held orbit key turns the camera, in degrees per second of fixed time. 90 degrees
/// per second is exactly 90/64 degrees per tick at the default 64 Hz, a value `f32` stores
/// exactly, so scripts can predict the angle after N ticks.
pub const ORBIT_DEGREES_PER_SECOND: f32 = 90.0;

/// Adds the orbit commands, actions and the system that moves the camera.
pub struct OrbitCameraModulePlugin;

impl Plugin for OrbitCameraModulePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Orbit>()
            .register_command(
                "orbit.start",
                "orbit.start <camera> <radius> <height>",
                start_command,
            )
            .register_command("orbit.stop", "orbit.stop", stop_command)
            .register_command(
                "orbit.set_angle",
                "orbit.set_angle <degrees>",
                set_angle_command,
            )
            .register_action(InputAction {
                id: "orbit_left",
                command_line: HELD_ONLY,
                default_keys: &[InputKey::ArrowLeft],
            })
            .register_action(InputAction {
                id: "orbit_right",
                command_line: HELD_ONLY,
                default_keys: &[InputKey::ArrowRight],
            })
            // After InputActionSystems: `is_held` is this tick's value on every platform.
            .add_systems(FixedUpdate, turn_orbit_camera.after(InputActionSystems));
    }
}

/// The running orbit, if any (a Bevy resource; `None` until `orbit.start`).
#[derive(Resource, Default, Debug, PartialEq)]
pub struct Orbit(pub Option<OrbitState>);

/// What the module needs to place the camera each tick.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitState {
    /// `Name` of the camera entity.
    pub camera: String,
    /// The point the camera circles (the camera's target when `orbit.start` ran).
    pub center: Vec3,
    pub radius: f32,
    pub height: f32,
    /// Current angle in degrees; 0 is on +z of the center, growing toward +x.
    pub angle_degrees: f32,
}

/// Where the camera sits for `state`: pure, so it is tested without an app.
pub fn orbit_position(state: &OrbitState) -> Vec3 {
    let angle = state.angle_degrees.to_radians();
    state.center
        + Vec3::new(
            angle.sin() * state.radius,
            state.height,
            angle.cos() * state.radius,
        )
}

/// Turns the angle by the held keys (right minus left) for one fixed tick, then places the camera.
fn turn_orbit_camera(
    actions: Res<InputActions>,
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
    mut cameras: Query<(&Name, &mut GameCamera)>,
) {
    let Some(state) = orbit.0.as_mut() else {
        return;
    };
    let direction =
        f32::from(actions.is_held("orbit_right")) - f32::from(actions.is_held("orbit_left"));
    state.angle_degrees += direction * ORBIT_DEGREES_PER_SECOND * time.delta_secs();
    for (name, mut camera) in &mut cameras {
        if name.as_str() == state.camera {
            camera.position = orbit_position(state);
        }
    }
}

/// `orbit.start <camera> <radius> <height>`.
fn start_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let camera_name: String = args.parse(0, "camera")?;
    let radius: f32 = args.parse(1, "radius")?;
    let height: f32 = args.parse(2, "height")?;
    if !(radius.is_finite() && radius > 0.0) {
        return Err(args.failure(format!("<radius> must be greater than 0, got {radius}")));
    }
    if !height.is_finite() {
        return Err(args.failure(format!("<height> must be a finite number, got {height}")));
    }
    let center = camera_target(world, args, &camera_name)?;
    let state = OrbitState {
        camera: camera_name,
        center,
        radius,
        height,
        angle_degrees: 0.0,
    };
    apply_state(world, args, state)
}

/// `orbit.stop`.
fn stop_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    world.resource_mut::<Orbit>().0 = None;
    Ok(CommandOutcome::Continue)
}

/// `orbit.set_angle <degrees>`.
fn set_angle_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let degrees: f32 = args.parse(0, "degrees")?;
    if !degrees.is_finite() {
        return Err(args.failure(format!("<degrees> must be a finite number, got {degrees}")));
    }
    let Some(mut state) = world.resource::<Orbit>().0.clone() else {
        return Err(args.failure(
            "no orbit is running; start one with `orbit.start <camera> <radius> <height>`",
        ));
    };
    state.angle_degrees = degrees;
    apply_state(world, args, state)
}

/// Stores `state` and moves the camera there now, so the effect shows without waiting a tick.
fn apply_state(world: &mut World, args: &CommandArgs, state: OrbitState) -> CommandResult {
    let position = orbit_position(&state);
    let mut cameras = world.query::<(&Name, &mut GameCamera)>();
    for (name, mut camera) in cameras.iter_mut(world) {
        if name.as_str() == state.camera {
            let mut moved = camera.clone();
            moved.position = position;
            moved.validate().map_err(|message| args.failure(message))?;
            *camera = moved;
        }
    }
    world.resource_mut::<Orbit>().0 = Some(state);
    Ok(CommandOutcome::Continue)
}

/// The target of the game camera named `name`; an error when there is no such camera.
fn camera_target(world: &mut World, args: &CommandArgs, name: &str) -> Result<Vec3, CommandError> {
    let mut cameras = world.query::<(&Name, &GameCamera)>();
    let mut found = cameras
        .iter(world)
        .filter(|(camera_name, _)| camera_name.as_str() == name)
        .map(|(_, camera)| camera.target);
    match (found.next(), found.next()) {
        (Some(target), None) => Ok(target),
        (None, _) => Err(args.failure(format!(
            "no game camera is named `{name}`; spawn an entity with a Name and a GameCamera first"
        ))),
        (Some(_), Some(_)) => Err(args.failure(format!(
            "more than one game camera is named `{name}`; give each camera a unique Name"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(angle_degrees: f32) -> OrbitState {
        OrbitState {
            camera: "cam".to_string(),
            center: Vec3::new(1.0, 0.0, 2.0),
            radius: 8.0,
            height: 4.0,
            angle_degrees,
        }
    }

    #[test]
    fn angle_zero_is_on_plus_z_and_ninety_is_on_plus_x() {
        assert!(orbit_position(&state(0.0)).distance(Vec3::new(1.0, 4.0, 10.0)) < 1e-5);
        assert!(orbit_position(&state(90.0)).distance(Vec3::new(9.0, 4.0, 2.0)) < 1e-5);
    }

    #[test]
    fn the_distance_to_the_center_stays_the_radius_at_any_angle() {
        for angle in [0.0, 33.0, 180.0, 271.5, 720.0] {
            let position = orbit_position(&state(angle));
            let flat = Vec2::new(position.x - 1.0, position.z - 2.0);
            assert!((flat.length() - 8.0).abs() < 1e-4, "angle {angle}");
            assert_eq!(position.y, 4.0);
        }
    }
}
