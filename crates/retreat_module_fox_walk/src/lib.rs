//! Fox walk module: decides *which clips* a model plays for a given speed; the engine plays them.
//!
//! The module only writes the contract's `AnimationPlayback` on the model entity. It never touches
//! Bevy's `AnimationPlayer` or graph, and has no timing code: clip time is simulation time owned by
//! the engine, so a blend driven by speed never restarts a clip.
//!
//! `fox.set_speed <model> <speed>` (speed in world units per second, 0 to [`MAX_SPEED`]):
//! - up to [`IDLE_MAX_SPEED`]: `Survey` (standing, looking around);
//! - then `Survey` blends into `Walk`, fully `Walk` at [`WALK_SPEED`];
//! - then `Walk` blends into `Run`, fully `Run` at [`RUN_SPEED`] and above.
//!
//! The clip names are the ones in Khronos' Fox.glb (`Survey`, `Walk`, `Run`); a model with
//! other names gets the engine's "no animation clip" error listing its own.

use bevy::prelude::*;
use retreat_engine_api::{
    AnimationPlayback, AppCommandExt, ClipPlayback, CommandArgs, CommandOutcome, CommandResult,
};

/// Fastest speed the command accepts.
pub const MAX_SPEED: f32 = 10.0;
/// At or below this speed the model only plays the idle clip.
pub const IDLE_MAX_SPEED: f32 = 0.1;
/// Speed at which the walk clip has full weight.
pub const WALK_SPEED: f32 = 1.0;
/// Speed at which the run clip has full weight.
pub const RUN_SPEED: f32 = 3.0;

const IDLE_CLIP: &str = "Survey";
const WALK_CLIP: &str = "Walk";
const RUN_CLIP: &str = "Run";

/// Adds `fox.set_speed`.
pub struct FoxWalkModulePlugin;

impl Plugin for FoxWalkModulePlugin {
    fn build(&self, app: &mut App) {
        app.register_command(
            "fox.set_speed",
            "fox.set_speed <model> <speed>",
            set_speed_command,
        );
    }
}

/// The looping clips (never a clip with weight 0) for `speed`: pure, so it is tested without an
/// app. Weights of the listed clips add up to 1.
pub fn clips_for_speed(speed: f32) -> Vec<ClipPlayback> {
    let weighted = |clip: &str, weight: f32| ClipPlayback {
        weight,
        ..ClipPlayback::looping(clip)
    };
    if speed <= IDLE_MAX_SPEED {
        vec![weighted(IDLE_CLIP, 1.0)]
    } else if speed < WALK_SPEED {
        let walk = (speed - IDLE_MAX_SPEED) / (WALK_SPEED - IDLE_MAX_SPEED);
        vec![weighted(IDLE_CLIP, 1.0 - walk), weighted(WALK_CLIP, walk)]
    } else if speed < RUN_SPEED {
        let run = (speed - WALK_SPEED) / (RUN_SPEED - WALK_SPEED);
        vec![weighted(WALK_CLIP, 1.0 - run), weighted(RUN_CLIP, run)]
    } else {
        vec![weighted(RUN_CLIP, 1.0)]
    }
}

/// `fox.set_speed <model> <speed>`.
fn set_speed_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(2)?;
    let model_name: String = args.parse(0, "model")?;
    let speed: f32 = args.parse(1, "speed")?;
    if !(speed.is_finite() && (0.0..=MAX_SPEED).contains(&speed)) {
        return Err(args.failure(format!(
            "<speed> must be from 0 to {MAX_SPEED}, got {speed}"
        )));
    }
    let mut names = world.query::<(Entity, &Name)>();
    let matches: Vec<Entity> = names
        .iter(world)
        .filter(|(_, name)| name.as_str() == model_name)
        .map(|(entity, _)| entity)
        .collect();
    let [model] = matches.as_slice() else {
        return Err(args.failure(format!(
            "expected exactly one entity named `{model_name}`, found {}; spawn the model first \
             (`model.spawn`) and give each entity a unique Name",
            matches.len()
        )));
    };
    world.entity_mut(*model).insert(AnimationPlayback {
        clips: clips_for_speed(speed),
    });
    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(speed: f32) -> Vec<(String, f32)> {
        clips_for_speed(speed)
            .into_iter()
            .map(|clip| (clip.clip, clip.weight))
            .collect()
    }

    #[test]
    fn standing_still_plays_only_survey() {
        assert_eq!(summary(0.0), vec![("Survey".to_string(), 1.0)]);
        assert_eq!(summary(IDLE_MAX_SPEED), vec![("Survey".to_string(), 1.0)]);
    }

    #[test]
    fn full_walk_and_full_run_are_single_clips() {
        assert_eq!(summary(WALK_SPEED)[0], ("Walk".to_string(), 1.0));
        assert_eq!(
            summary(WALK_SPEED).len(),
            2,
            "walk blends toward run from here"
        );
        assert_eq!(summary(RUN_SPEED), vec![("Run".to_string(), 1.0)]);
        assert_eq!(summary(MAX_SPEED), vec![("Run".to_string(), 1.0)]);
    }

    #[test]
    fn blends_have_weights_that_add_up_to_one_and_follow_speed() {
        for speed in [0.2, 0.55, 0.99, 1.0, 1.7, 2.9] {
            let total: f32 = clips_for_speed(speed).iter().map(|clip| clip.weight).sum();
            assert!((total - 1.0).abs() < 1e-5, "speed {speed}: {total}");
        }
        let halfway = summary(2.0);
        assert_eq!(halfway[0].0, "Walk");
        assert!((halfway[0].1 - 0.5).abs() < 1e-6);
        assert_eq!(halfway[1].0, "Run");
    }

    #[test]
    fn every_listed_clip_loops_and_the_list_is_valid() {
        for speed in [0.0, 0.5, 1.0, 2.0, 5.0] {
            let playback = AnimationPlayback {
                clips: clips_for_speed(speed),
            };
            assert_eq!(playback.validate(), Ok(()), "speed {speed}");
            assert!(playback.clips.iter().all(|clip| clip.looping));
        }
    }
}
