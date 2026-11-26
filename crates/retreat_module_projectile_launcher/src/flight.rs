//! Rockets flying and exploding. Runs on every machine (clients predict); damage only where the
//! simulation is authoritative.

use crate::config::LauncherConfig;
use crate::state::Rocket;
use bevy::math::ops;
use bevy::prelude::*;
use retreat_engine_api::{
    NetLocalClient, NetPose, PhysicsShape, RayCast, RemoveNetEntity, RetiredEntity, ShapeCast,
    SoundEvents, VisualEffect, VisualEffects, cast_ray, cast_shape,
};
use retreat_gameplay_api::{
    CharacterSize, DamageCause, DamageRequested, PlayerId, event_time, ticks_from_seconds,
};
use retreat_weapon_kit::{character_of, dead_players};

/// An exploded rocket stays this long, so every client hears of the explosion.
pub const LINGER_TICKS: u32 = 16;
/// A rocket this young starts its launch sound (a client hears of another's rocket a few ticks
/// late).
const LAUNCH_SOUND_TICKS: u32 = 16;
const CORE_COLOR: Color = Color::srgb(1.0, 0.95, 0.6);
const FIRE_COLOR: Color = Color::srgb(1.0, 0.5, 0.1);
const EMBER_COLOR: Color = Color::srgb(0.9, 0.2, 0.05);
/// Where the puffs around an explosion are, in blast radii.
const PUFFS: [Vec3; 6] = [
    Vec3::new(0.55, 0.15, 0.1),
    Vec3::new(-0.5, 0.25, -0.2),
    Vec3::new(0.1, 0.6, -0.15),
    Vec3::new(-0.15, 0.2, 0.55),
    Vec3::new(0.2, 0.1, -0.55),
    Vec3::new(-0.3, 0.45, 0.25),
];

/// One glowing sphere of an explosion.
#[derive(Debug, Clone, PartialEq)]
pub struct BlastPart {
    /// The end of the effect's id.
    pub name: String,
    /// From the center of the explosion.
    pub offset: Vec3,
    pub radius: f32,
    pub color: Color,
    pub ticks: u32,
}

/// What an explosion of `radius` looks like: a short bright core, a ball of fire and puffs around
/// it that stay a little longer. The puffs end inside the blast radius.
pub fn blast_look(radius: f32) -> Vec<BlastPart> {
    let part = |name: String, offset, size: f32, color, ticks| BlastPart {
        name,
        offset,
        radius: radius * size,
        color,
        ticks,
    };
    let mut parts = vec![
        part("core".to_string(), Vec3::ZERO, 0.3, CORE_COLOR, 8),
        part("fire".to_string(), Vec3::ZERO, 0.5, FIRE_COLOR, 16),
    ];
    parts.extend(PUFFS.iter().enumerate().map(|(puff, direction)| {
        let ticks = 18 + 3 * (puff as u32 % 3);
        part(
            format!("puff{puff}"),
            *direction * radius,
            0.22,
            EMBER_COLOR,
            ticks,
        )
    }));
    parts
}

/// How far `point` is from the capsule of a character whose middle is at `center`; 0 inside it.
pub fn distance_to_body(point: Vec3, center: Vec3, size: CharacterSize) -> f32 {
    let half = size.capsule_half_height();
    let nearest = center.y + (point.y - center.y).clamp(-half, half);
    let from_axis = point.distance(Vec3::new(center.x, nearest, center.z));
    (from_axis - size.radius).max(0.0)
}

/// Damage at `distance` from the body: `damage` at 0, falling in a straight line to `edge_damage`
/// at the blast radius, nothing beyond it.
pub fn blast_damage(config: &LauncherConfig, distance: f32) -> u32 {
    if distance > config.blast_radius {
        return 0;
    }
    let share = (distance / config.blast_radius).clamp(0.0, 1.0);
    let fall = (config.damage - config.edge_damage) as f32 * share;
    config.damage - ops::round(fall) as u32
}

/// Moves every rocket in name order, explodes it on what it meets, and removes it later.
pub(crate) fn fly_rockets(world: &mut World) {
    let mut query =
        world.query_filtered::<(Entity, &Name, &Rocket, &NetPose), Without<RetiredEntity>>();
    let mut rockets: Vec<(String, Entity, Rocket, NetPose)> = query
        .iter(world)
        .map(|(entity, name, rocket, pose)| (name.to_string(), entity, *rocket, *pose))
        .collect();
    if rockets.is_empty() {
        return;
    }
    rockets.sort_by(|left, right| left.0.cmp(&right.0));
    let timestep = world.resource::<Time<Fixed>>().timestep();
    let config = world.resource::<LauncherConfig>().clone();
    let lifetime = ticks_from_seconds(config.lifetime_seconds, timestep).max(1);
    for (name, entity, mut rocket, mut pose) in rockets {
        if rocket.exploded > 0 {
            rocket.exploded += 1;
            if rocket.exploded > LINGER_TICKS {
                world.entity_mut(entity).insert(RemoveNetEntity);
            }
        } else {
            rocket.age += 1;
            if rocket.age <= LAUNCH_SOUND_TICKS
                && let Some(cue) = &config.launch_sound
            {
                let time = event_time(*world.resource::<NetLocalClient>(), rocket.owner);
                let event = cue.event(format!("{name}/launch"), Some(pose.translation), time);
                world.resource_mut::<SoundEvents>().play(event);
            }
            let step = config.speed * timestep.as_secs_f32();
            let mut exclude = dead_players(world);
            exclude.extend(character_of(world, rocket.owner));
            let hit = cast_shape(
                world,
                &ShapeCast {
                    shape: PhysicsShape::Ball {
                        radius: config.rocket_radius,
                    },
                    origin: pose.translation,
                    rotation: Quat::IDENTITY,
                    direction: rocket.direction,
                    max_distance: step,
                    exclude,
                },
            );
            pose.translation += rocket.direction * hit.as_ref().map_or(step, |hit| hit.distance);
            if hit.is_some() || rocket.age >= lifetime {
                rocket.exploded = 1;
                let direct = hit.map(|hit| hit.entity);
                explode(world, &name, rocket.owner, pose.translation, direct);
            }
        }
        world.entity_mut(entity).insert((rocket, pose));
    }
}

/// The look and sound everywhere, the damage where this machine decides it.
fn explode(world: &mut World, name: &str, owner: PlayerId, at: Vec3, direct: Option<Entity>) {
    let client = *world.resource::<NetLocalClient>();
    let local = client.0;
    // Another player's rocket is drawn in the past, so its explosion waits for the view too.
    let time = event_time(client, owner);
    let config = world.resource::<LauncherConfig>();
    let (radius, cue) = (config.blast_radius, config.explosion_sound.clone());
    for part in blast_look(radius) {
        world.resource_mut::<VisualEffects>().play(VisualEffect {
            id: format!("{name}/boom/{}", part.name),
            position: at + part.offset,
            radius: part.radius,
            color: part.color,
            glow: 2.5,
            ticks: part.ticks,
            time,
        });
    }
    if let Some(cue) = cue {
        let event = cue.event(format!("{name}/boom"), Some(at), time);
        world.resource_mut::<SoundEvents>().play(event);
    }
    if local.is_none() {
        for request in blast_requests(world, owner, at, direct) {
            world.write_message(request);
        }
    }
}

/// Who the explosion hurts and how much, in player order.
fn blast_requests(
    world: &mut World,
    owner: PlayerId,
    at: Vec3,
    direct: Option<Entity>,
) -> Vec<DamageRequested> {
    let mut query =
        world.query_filtered::<(Entity, &PlayerId, &Transform), Without<RetiredEntity>>();
    let mut players: Vec<(PlayerId, Entity, Vec3)> = query
        .iter(world)
        .map(|(entity, player, transform)| (*player, entity, transform.translation))
        .collect();
    players.sort_by_key(|(player, ..)| *player);
    let everyone: Vec<Entity> = players.iter().map(|(_, entity, _)| *entity).collect();
    let dead = dead_players(world);
    let size = *world.resource::<CharacterSize>();
    let config = world.resource::<LauncherConfig>().clone();
    let mut requests = Vec::new();
    for (player, entity, center) in players {
        if dead.contains(&entity) {
            continue;
        }
        let direct = direct == Some(entity);
        let distance = if direct {
            0.0
        } else {
            distance_to_body(at, center, size)
        };
        let mut amount = blast_damage(&config, distance);
        if amount == 0 || (!direct && behind_a_wall(world, at, center, &everyone)) {
            continue;
        }
        if player == owner {
            amount = amount * config.self_damage_percent / 100;
        }
        requests.push(DamageRequested {
            target: player,
            amount,
            attacker: Some(owner),
            cause: if direct {
                DamageCause::Projectile
            } else {
                DamageCause::Blast
            },
        });
    }
    requests
}

/// Whether something that is no character is between the explosion and the middle of the body.
fn behind_a_wall(world: &mut World, from: Vec3, center: Vec3, characters: &[Entity]) -> bool {
    let to = center - from;
    let Some(direction) = to.try_normalize() else {
        return false;
    };
    let cast = RayCast {
        origin: from,
        direction,
        max_distance: to.length(),
        exclude: characters.to_vec(),
    };
    cast_ray(world, &cast).is_some()
}
