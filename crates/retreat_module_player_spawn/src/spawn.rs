//! Creating and removing characters.

use bevy::prelude::*;
use retreat_engine_api::{NetClientJoined, NetClientLeft, PhysicsBody, RemovePhysicsBody};
use retreat_gameplay_api::{
    CharacterSize, LookAngles, MAX_PLAYERS, PlayerId, SpawnPoint, SpawnPoints, choose_spawn_point,
    player_entity_name,
};

const CHARACTER_COLOR: Color = Color::srgb(0.9, 0.35, 0.25);

/// Spawns the character of `player` at the spawn point farthest from the other characters. The
/// error says why it cannot exist.
pub fn spawn_character(world: &mut World, player: PlayerId) -> Result<(), String> {
    let occupied = occupied_feet(world);
    let points = &world.resource::<SpawnPoints>().0;
    let Some(index) = choose_spawn_point(points, &occupied) else {
        return Err("there are no spawn points; load a map first".to_string());
    };
    let spawn = points[index];
    spawn_character_at(world, player, spawn).map(|_| ())
}

/// Where the feet of every character are.
pub(crate) fn occupied_feet(world: &mut World) -> Vec<Vec3> {
    let size = *world.resource::<CharacterSize>();
    let mut characters = world.query_filtered::<&Transform, With<PlayerId>>();
    characters
        .iter(world)
        .map(|transform| size.feet(transform.translation))
        .collect()
}

/// Spawns the character of `player` with its feet at `spawn`, looking its way; returns the body.
/// The error says why it cannot exist.
pub fn spawn_character_at(
    world: &mut World,
    player: PlayerId,
    spawn: SpawnPoint,
) -> Result<Entity, String> {
    let name = player_entity_name(player);
    let size = *world.resource::<CharacterSize>();
    let mut existing = world.query::<(&Name, Option<&PlayerId>)>();
    let mut players = 0;
    for (found, other) in existing.iter(world) {
        // A character that left keeps its name for the engine's rollback window.
        if found.as_str() == name {
            return Err(format!("an entity named `{name}` already exists"));
        }
        if other.is_some() {
            players += 1;
        }
    }
    if players >= MAX_PLAYERS {
        return Err(format!("there are already {MAX_PLAYERS} players"));
    }
    let mesh = capsule_mesh(world, size);
    let mut character = world.spawn((
        Name::new(name),
        player,
        LookAngles {
            yaw: spawn.yaw,
            pitch: 0.0,
        },
        PhysicsBody::character(size.capsule_half_height(), size.radius),
        Transform::from_translation(size.center(spawn.feet)),
    ));
    if let Some(mesh) = mesh {
        character.insert(mesh);
    }
    Ok(character.id())
}

/// A visible capsule, or nothing in an app without mesh assets.
fn capsule_mesh(
    world: &mut World,
    size: CharacterSize,
) -> Option<(Mesh3d, MeshMaterial3d<StandardMaterial>)> {
    let mesh = world
        .get_resource_mut::<Assets<Mesh>>()?
        .add(Capsule3d::new(
            size.radius,
            size.capsule_half_height() * 2.0,
        ));
    let material = world
        .get_resource_mut::<Assets<StandardMaterial>>()?
        .add(CHARACTER_COLOR);
    Some((Mesh3d(mesh), MeshMaterial3d(material)))
}

pub(crate) fn spawn_joined_clients(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<NetClientJoined>>,
) {
    // The cursor only remembers which messages were read; it holds no game state.
    let mut joined: Vec<u32> = cursor
        .read(world.resource::<Messages<NetClientJoined>>())
        .map(|message| message.client_id)
        .collect();
    joined.sort_unstable();
    for client_id in joined {
        if let Err(problem) = spawn_character(world, PlayerId(client_id)) {
            warn!("player_spawn: client {client_id} gets no character: {problem}");
        }
    }
}

pub(crate) fn remove_left_clients(
    mut left: MessageReader<NetClientLeft>,
    players: Query<(Entity, &PlayerId)>,
    mut commands: Commands,
) {
    for message in left.read() {
        for (entity, player) in &players {
            if player.0 == message.client_id {
                // Retired, not despawned: a client may still rewind past this tick.
                commands.entity(entity).insert(RemovePhysicsBody);
            }
        }
    }
}
