//! High score module: decides *what* is saved (a score and a best score); the engine decides
//! where (file, browser storage) through the contract's `SaveStore`.
//!
//! - `score.add <n>`: adds `n` points (0 to [`MAX_POINTS_PER_ADD`]); the best score follows.
//! - `score.reset`: score back to 0 (the best score stays).
//! - `score.save`: saves `{ "score": .., "best": .. }` as the module `highscore`, data version
//!   [`DATA_VERSION`].
//! - `score.load`: loads it back. Nothing saved is not an error (a first run). Data from an older
//!   version is migrated ([`migrate`]); data from a newer version is refused with a message.
//! - `score.show`: logs the score.
//!
//! Version history: version 1 saved `{ "points": n }` (one number, no best score); version 2 (now)
//! saves `{ "score": n, "best": m }`.

use bevy::prelude::*;
use retreat_engine_api::serde_json::{Value, json};
use retreat_engine_api::{
    AppCommandExt, CommandArgs, CommandError, CommandOutcome, CommandResult, SaveError, SaveStore,
    SavedData,
};

/// The id this module saves under.
pub const MODULE_ID: &str = "highscore";
/// The data version this module writes and the newest it can read.
pub const DATA_VERSION: u32 = 2;
/// Most points one `score.add` may give (a typo like `score.add 99999999999` stays harmless).
pub const MAX_POINTS_PER_ADD: u32 = 1_000_000;

/// Adds the score resource and the commands.
pub struct HighscoreModulePlugin;

impl Plugin for HighscoreModulePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .register_command("score.add", "score.add <points>", add_command)
            .register_command("score.reset", "score.reset", reset_command)
            .register_command("score.save", "score.save", save_command)
            .register_command("score.load", "score.load", load_command)
            .register_command("score.show", "score.show", show_command);
    }
}

/// The current and best score.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub current: u64,
    pub best: u64,
}

/// The JSON data of a [`Score`], at [`DATA_VERSION`].
pub fn score_to_data(score: Score) -> Value {
    json!({ "score": score.current, "best": score.best })
}

/// Turns saved data of any version up to [`DATA_VERSION`] into a [`Score`]. Pure, so every
/// version is unit tested.
pub fn migrate(version: u32, data: &Value) -> Result<Score, String> {
    let number = |field: &str| -> Result<u64, String> {
        data.get(field)
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("version {version} data has no whole number `{field}`: {data}"))
    };
    match version {
        // Version 1: one number, called `points`; the best score did not exist yet.
        1 => {
            let points = number("points")?;
            Ok(Score {
                current: points,
                best: points,
            })
        }
        DATA_VERSION => Ok(Score {
            current: number("score")?,
            best: number("best")?,
        }),
        other => Err(format!(
            "the highscore module has no data version {other} (it knows 1 to {DATA_VERSION})"
        )),
    }
}

/// `score.add <points>`.
fn add_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let points: u32 = args.parse(0, "points")?;
    if points > MAX_POINTS_PER_ADD {
        return Err(args.failure(format!(
            "<points> must be from 0 to {MAX_POINTS_PER_ADD}, got {points}"
        )));
    }
    let mut score = world.resource_mut::<Score>();
    score.current += u64::from(points);
    score.best = score.best.max(score.current);
    Ok(CommandOutcome::Continue)
}

/// `score.reset`.
fn reset_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    world.resource_mut::<Score>().current = 0;
    Ok(CommandOutcome::Continue)
}

fn save_error(args: &CommandArgs, error: SaveError) -> CommandError {
    args.failure(error.to_string())
}

/// `score.save`.
fn save_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let data = score_to_data(*world.resource::<Score>());
    world
        .resource_mut::<SaveStore>()
        .save(MODULE_ID, DATA_VERSION, &data)
        .map_err(|error| save_error(args, error))?;
    Ok(CommandOutcome::Continue)
}

/// `score.load`.
fn load_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let loaded: Option<SavedData> = world
        .resource::<SaveStore>()
        .load(MODULE_ID)
        .map_err(|error| save_error(args, error))?;
    let Some(saved) = loaded else {
        info!("highscore: no save yet, keeping the current score");
        return Ok(CommandOutcome::Continue);
    };
    saved
        .check_not_newer_than(MODULE_ID, DATA_VERSION)
        .map_err(|error| save_error(args, error))?;
    let score = migrate(saved.version, &saved.data).map_err(|message| {
        args.failure(format!(
            "the saved highscore cannot be read: {message}; `save.delete {MODULE_ID}` removes it"
        ))
    })?;
    *world.resource_mut::<Score>() = score;
    Ok(CommandOutcome::Continue)
}

/// `score.show`.
fn show_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let score = world.resource::<Score>();
    info!("highscore: score {}, best {}", score.current, score.best);
    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_data_round_trips() {
        let score = Score {
            current: 40,
            best: 90,
        };
        assert_eq!(migrate(DATA_VERSION, &score_to_data(score)), Ok(score));
    }

    #[test]
    fn version_one_data_is_migrated_with_best_equal_to_points() {
        let migrated = migrate(1, &json!({"points": 75}));
        assert_eq!(
            migrated,
            Ok(Score {
                current: 75,
                best: 75
            })
        );
    }

    #[test]
    fn bad_data_and_unknown_versions_say_what_is_wrong() {
        let error = migrate(2, &json!({"score": 1})).unwrap_err();
        assert!(error.contains("no whole number `best`"), "{error}");
        let error = migrate(1, &json!({"score": 1})).unwrap_err();
        assert!(error.contains("no whole number `points`"), "{error}");
        let error = migrate(0, &json!({})).unwrap_err();
        assert!(error.contains("no data version 0"), "{error}");
    }
}
