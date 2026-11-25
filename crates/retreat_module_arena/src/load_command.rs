//! `arena.load <map>`.

use crate::MAPS;
use crate::floor::floor_tiles;
use crate::map::{MapPiece, PieceKind, validate_map};
use bevy::prelude::*;
use retreat_engine_api::{
    CommandArgs, CommandOutcome, CommandResult, PhysicsBody, PhysicsBodyKind, PhysicsShape,
};
use retreat_gameplay_api::SpawnPoints;

/// Marks an entity built from a map piece.
#[derive(Component)]
pub struct ArenaPiece(pub PieceKind);

/// The id of the loaded map; absent until `arena.load` succeeds.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct LoadedArena(pub String);

/// The entity name of a map piece: `arena/<piece>`.
pub fn piece_entity_name(piece: &str) -> String {
    format!("arena/{piece}")
}

pub(crate) fn load_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let id: String = args.parse(0, "map")?;
    let Some((_, build)) = MAPS.iter().find(|(name, _)| *name == id) else {
        let known: Vec<&str> = MAPS.iter().map(|(name, _)| *name).collect();
        return Err(args.failure(format!(
            "unknown map `{id}`; known maps: {}",
            known.join(", ")
        )));
    };
    if let Some(loaded) = world.get_resource::<LoadedArena>() {
        return Err(args.failure(format!(
            "map `{}` is already loaded; one map per session",
            loaded.0
        )));
    }
    let map = build();
    let problems = validate_map(&map);
    if !problems.is_empty() {
        return Err(args.failure(format!("map `{id}` is invalid: {}", problems.join("; "))));
    }
    let mut names = world.query::<&Name>();
    let taken = names.iter(world).map(Name::as_str).find(|existing| {
        map.pieces
            .iter()
            .any(|piece| piece_entity_name(&piece.name) == *existing)
    });
    if let Some(taken) = taken {
        return Err(args.failure(format!("an entity named `{taken}` already exists")));
    }
    for piece in &map.pieces {
        spawn_piece(world, piece);
    }
    world.insert_resource(SpawnPoints(map.spawn_points));
    world.insert_resource(LoadedArena(id));
    Ok(CommandOutcome::Continue)
}

fn spawn_piece(world: &mut World, piece: &MapPiece) {
    let size = piece.size();
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_size(size));
    let surface = piece_surface(world, piece.kind);
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(surface);
    world.spawn((
        Name::new(piece_entity_name(&piece.name)),
        ArenaPiece(piece.kind),
        PhysicsBody {
            kind: PhysicsBodyKind::Fixed,
            shape: PhysicsShape::Cuboid {
                half_extents: size / 2.0,
            },
            shape_offset: Vec3::ZERO,
        },
        Transform::from_translation(piece.center()),
        Mesh3d(mesh),
        MeshMaterial3d(material),
    ));
}

/// The material of a piece. The floor is tiled where the app has image assets.
fn piece_surface(world: &mut World, kind: PieceKind) -> StandardMaterial {
    let tiles = (kind == PieceKind::Floor)
        .then(|| {
            world
                .get_resource_mut::<Assets<Image>>()
                .map(|mut images| images.add(floor_tiles()))
        })
        .flatten();
    StandardMaterial {
        // A texture is multiplied by the base color, and the tiles carry their own colors.
        base_color: if tiles.is_some() {
            Color::WHITE
        } else {
            graybox_color(kind)
        },
        base_color_texture: tiles,
        perceptual_roughness: 0.9,
        ..default()
    }
}

fn graybox_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::Floor => Color::srgb(0.7, 0.7, 0.7),
        PieceKind::Wall | PieceKind::Pillar => Color::srgb(0.45, 0.5, 0.6),
        PieceKind::Platform => Color::srgb(0.55, 0.7, 0.55),
        PieceKind::Stair => Color::srgb(0.7, 0.7, 0.75),
        PieceKind::Cover => Color::srgb(0.8, 0.55, 0.3),
    }
}
