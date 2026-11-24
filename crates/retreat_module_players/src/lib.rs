//! Players module: a body for every connected client, moved by that client's held keys.
//!
//! The module never touches the transport, Rapier or any platform code; it uses only the contract:
//! - **Server:** on `NetClientJoined` it spawns the dynamic cube `player_<id>` at a spawn point chosen
//!   from the id; on `NetClientLeft` it despawns it. (Other machines learn about the body through the
//!   engine's state replication, and lose it the same way.)
//! - **Command** `player.move <x> <z>`: adds a velocity change of `(x, 0, z)` to the body of the player
//!   who sent the command (`NetCommandSender`); the server allows clients to send it
//!   (`allow_client_command`). The change is clamped to [`MAX_MOVE_PER_COMMAND`] per axis so a client
//!   cannot launch its body.
//! - **Client:** four held input actions (arrow keys, `W`/`A`/`S`/`D` are free for other modules) turn into
//!   one `player.move` line per tick while a key is held, queued in `NetInputQueue`; the engine runs the
//!   line on the client (prediction) and sends it to the server.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, AppNetExt, CommandArgs, CommandOutcome, CommandResult, HELD_ONLY,
    InputAction, InputActionSystems, InputActions, InputKey, NetClientJoined, NetClientLeft,
    NetCommandSender, NetInputQueue, PhysicsBody, PhysicsVelocityChange, add_net_contract,
};

/// Largest velocity change per axis one `player.move` may apply (units per second).
pub const MAX_MOVE_PER_COMMAND: f32 = 0.5;
/// Velocity change per axis a client queues each tick while a key is held (units per second per tick).
pub const HELD_MOVE_PER_TICK: f32 = 0.03125;
/// Height at which players appear.
const SPAWN_HEIGHT: f32 = 0.6;
/// Distance between neighbouring spawn points (small enough that a row fits on the sandbox ground).
const SPAWN_SPACING: f32 = 1.5;
/// Spawn points per row.
const SPAWN_PER_ROW: u32 = 5;

/// Adds the player commands, actions and systems.
pub struct PlayersModulePlugin;

impl Plugin for PlayersModulePlugin {
    fn build(&self, app: &mut App) {
        add_net_contract(app);
        app.register_command("player.move", "player.move <x> <z>", move_command)
            .allow_client_command("player.move")
            .register_action(held_action("player_left", InputKey::ArrowLeft))
            .register_action(held_action("player_right", InputKey::ArrowRight))
            .register_action(held_action("player_forward", InputKey::ArrowUp))
            .register_action(held_action("player_back", InputKey::ArrowDown))
            .add_systems(
                FixedUpdate,
                (
                    spawn_players_for_new_clients,
                    remove_players_of_left_clients,
                    queue_held_movement.after(InputActionSystems),
                ),
            );
    }
}

fn held_action(id: &'static str, key: InputKey) -> InputAction {
    InputAction {
        id,
        command_line: HELD_ONLY,
        default_keys: match key {
            InputKey::ArrowLeft => &[InputKey::ArrowLeft],
            InputKey::ArrowRight => &[InputKey::ArrowRight],
            InputKey::ArrowUp => &[InputKey::ArrowUp],
            _ => &[InputKey::ArrowDown],
        },
    }
}

/// The entity name of client `id`'s player.
pub fn player_name(client_id: u32) -> String {
    format!("player_{client_id}")
}

/// Where client `id`'s player appears: a grid of spawn points, so players do not start inside each other.
pub fn spawn_position(client_id: u32) -> Vec3 {
    let column = (client_id % SPAWN_PER_ROW) as f32;
    let row = (client_id / SPAWN_PER_ROW) as f32;
    Vec3::new(
        (column - 2.0) * SPAWN_SPACING,
        SPAWN_HEIGHT,
        2.0 + row * SPAWN_SPACING,
    )
}

/// The movement a client wants from its held keys: `(x, z)` per tick, each in -1..=1 scaled by
/// [`HELD_MOVE_PER_TICK`]. Pure, so it is tested without an app.
pub fn held_movement(left: bool, right: bool, forward: bool, back: bool) -> (f32, f32) {
    let axis = |negative: bool, positive: bool| match (negative, positive) {
        (true, false) => -HELD_MOVE_PER_TICK,
        (false, true) => HELD_MOVE_PER_TICK,
        _ => 0.0,
    };
    (axis(left, right), axis(forward, back))
}

fn spawn_players_for_new_clients(
    mut joined: MessageReader<NetClientJoined>,
    mut commands: Commands,
) {
    for message in joined.read() {
        commands.spawn((
            Name::new(player_name(message.client_id)),
            PhysicsBody::dynamic_cube(1.0),
            Transform::from_translation(spawn_position(message.client_id)),
        ));
    }
}

fn remove_players_of_left_clients(
    mut left: MessageReader<NetClientLeft>,
    players: Query<(Entity, &Name)>,
    mut commands: Commands,
) {
    for message in left.read() {
        let wanted = player_name(message.client_id);
        for (entity, name) in &players {
            if name.as_str() == wanted {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Client side: one `player.move` line per tick while a movement key is held.
fn queue_held_movement(actions: Res<InputActions>, mut queue: ResMut<NetInputQueue>) {
    let (x, z) = held_movement(
        actions.is_held("player_left"),
        actions.is_held("player_right"),
        actions.is_held("player_forward"),
        actions.is_held("player_back"),
    );
    if x != 0.0 || z != 0.0 {
        queue.push(format!("player.move {x} {z}"));
    }
}

/// `player.move <x> <z>`.
fn move_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let x: f32 = args.parse(0, "x")?;
    let z: f32 = args.parse(1, "z")?;
    if !(x.is_finite() && z.is_finite()) {
        return Err(args.failure(format!(
            "<x> and <z> must be finite numbers, got {x} and {z}"
        )));
    }
    let Some(client_id) = world.resource::<NetCommandSender>().0 else {
        return Err(args.failure(
            "this command moves the player who sent it; run it from a network client with `net.send player.move <x> <z>`",
        ));
    };
    let wanted = player_name(client_id);
    let mut players = world.query::<(Entity, &Name)>();
    let player = players
        .iter(world)
        .find(|(_, name)| name.as_str() == wanted)
        .map(|(entity, _)| entity);
    let Some(player) = player else {
        return Err(args.failure(format!(
            "there is no body `{wanted}` yet (it is created when the client joins)"
        )));
    };
    let change = Vec3::new(
        x.clamp(-MAX_MOVE_PER_COMMAND, MAX_MOVE_PER_COMMAND),
        0.0,
        z.clamp(-MAX_MOVE_PER_COMMAND, MAX_MOVE_PER_COMMAND),
    );
    // Several commands in one tick add up.
    let mut entity = world.entity_mut(player);
    let total = entity
        .get::<PhysicsVelocityChange>()
        .map_or(change, |pending| pending.0 + change);
    entity.insert(PhysicsVelocityChange(total));
    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_keys_make_a_movement_and_opposite_keys_cancel() {
        assert_eq!(held_movement(false, false, false, false), (0.0, 0.0));
        assert_eq!(
            held_movement(true, false, false, false),
            (-HELD_MOVE_PER_TICK, 0.0)
        );
        assert_eq!(
            held_movement(false, true, true, false),
            (HELD_MOVE_PER_TICK, -HELD_MOVE_PER_TICK)
        );
        assert_eq!(held_movement(true, true, true, true), (0.0, 0.0));
    }

    #[test]
    fn players_spawn_on_distinct_points() {
        let points: Vec<Vec3> = (0..12).map(spawn_position).collect();
        for (index, point) in points.iter().enumerate() {
            assert!(
                points[index + 1..]
                    .iter()
                    .all(|other| other.distance(*point) > 1.0)
            );
            assert_eq!(point.y, SPAWN_HEIGHT);
        }
    }
}
