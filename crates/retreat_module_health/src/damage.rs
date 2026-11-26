//! Losing hit points and dying.

use crate::config::HealthConfig;
use crate::state::SpawnProtection;
use bevy::prelude::*;
use retreat_engine_api::{Intangible, RetiredEntity};
use retreat_gameplay_api::{
    DamageApplied, DamageCause, DamageRequested, HealRequested, Health, MatchState, PlayerDied,
    PlayerId, RespawnTimer, ticks_from_seconds,
};

/// "This player dies now", whatever its health or protection (`health.kill`, falling out).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct KillRequested(pub PlayerId);

type Targets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerId,
        &'static mut Health,
        Has<SpawnProtection>,
    ),
    Without<RetiredEntity>,
>;

type Matches<'w, 's> = Query<'w, 's, &'static MatchState, Without<RetiredEntity>>;

/// Asks to kill every living player below the fall-out height.
pub(crate) fn request_fall_outs(
    players: Query<(&PlayerId, &Health, &Transform), Without<RetiredEntity>>,
    config: Res<HealthConfig>,
    mut kills: MessageWriter<KillRequested>,
) {
    let mut fallen: Vec<PlayerId> = players
        .iter()
        .filter(|(_, health, transform)| {
            health.is_alive() && transform.translation.y < config.fall_out_height
        })
        .map(|(player, ..)| *player)
        .collect();
    fallen.sort();
    for player in fallen {
        kills.write(KillRequested(player));
    }
}

pub(crate) fn apply_kills(
    mut requests: MessageReader<KillRequested>,
    mut players: Targets,
    matches: Matches,
    mut outcome: Outcome,
) {
    let combat = matches.iter().all(MatchState::allows_combat);
    for KillRequested(target) in requests.read().copied() {
        for (entity, player, mut health, _) in &mut players {
            if *player == target && combat && health.is_alive() {
                let all = health.current();
                let request = DamageRequested {
                    target,
                    amount: all,
                    attacker: None,
                    cause: DamageCause::World,
                };
                outcome.hurt(entity, &mut health, request);
            }
        }
    }
}

pub(crate) fn apply_damage(
    mut requests: MessageReader<DamageRequested>,
    mut players: Targets,
    matches: Matches,
    mut outcome: Outcome,
) {
    let combat = matches.iter().all(MatchState::allows_combat);
    for request in requests.read().copied() {
        for (entity, player, mut health, protected) in &mut players {
            if *player == request.target && combat && !protected && health.is_alive() {
                outcome.hurt(entity, &mut health, request);
            }
        }
    }
}

/// Restores what used items give, before this tick's damage.
pub(crate) fn apply_heals(mut requests: MessageReader<HealRequested>, mut players: Targets) {
    for request in requests.read().copied() {
        for (_, player, mut health, _) in &mut players {
            if *player == request.player && health.is_alive() {
                let mut healed = *health;
                healed.heal(request.amount);
                health.set_if_neq(healed);
            }
        }
    }
}

/// What damage causes: the messages, and the respawn timer of the dead.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Outcome<'w, 's> {
    config: Res<'w, HealthConfig>,
    time: Res<'w, Time<Fixed>>,
    applied: MessageWriter<'w, DamageApplied>,
    died: MessageWriter<'w, PlayerDied>,
    commands: Commands<'w, 's>,
}

impl Outcome<'_, '_> {
    fn hurt(&mut self, entity: Entity, health: &mut Mut<Health>, request: DamageRequested) {
        if request.amount == 0 {
            return;
        }
        let result = health.apply_damage(request.amount);
        self.applied.write(DamageApplied {
            target: request.target,
            amount: result.applied,
            attacker: request.attacker,
            cause: request.cause,
            remaining: health.current(),
        });
        if result.killed {
            self.died.write(PlayerDied {
                victim: request.target,
                killer: request.attacker,
                cause: request.cause,
            });
            // At least one tick: a respawn never happens in the tick of the death.
            let delay =
                ticks_from_seconds(self.config.respawn_delay_seconds, self.time.timestep()).max(1);
            self.commands
                .entity(entity)
                // The dead are walked through and casts ignore them until they respawn.
                .insert((RespawnTimer { ticks_left: delay }, Intangible))
                .remove::<SpawnProtection>();
        }
    }
}
