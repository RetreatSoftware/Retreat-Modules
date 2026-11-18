//! Sound effects module: decides *which* sound plays *when*; the engine plays it.
//!
//! The module only spawns the contract's `SoundPlayback` on a named entity. It has no audio
//! library, no platform code and no knowledge of speakers or browsers: whether a sound is
//! playing is decided by the engine from simulation time.
//!
//! - `sfx.play <path>`: plays a one-shot effect named `sfx_<n>` (`sfx_1`, `sfx_2`, ...).
//! - `sfx.music <path>|off`: starts the looping music `sfx_music` (replacing the old one), or
//!   stops it with `off`.
//! - Action `sfx_beep` (key P): runs `sfx.play sounds/beep.wav`, the sandbox's test sound. A game
//!   that uses this module rebinds it with `input.bind` or ships its own sound path.

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppInputExt, CommandArgs, CommandOutcome, CommandResult, InputAction, InputKey,
    SoundPlayback,
};

/// Name of the music entity.
pub const MUSIC_ENTITY_NAME: &str = "sfx_music";

/// Adds the sound commands and the beep action.
pub struct SfxModulePlugin;

impl Plugin for SfxModulePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectCount>()
            .register_command("sfx.play", "sfx.play <path>", play_command)
            .register_command("sfx.music", "sfx.music <path>|off", music_command)
            .register_action(InputAction {
                id: "sfx_beep",
                command_line: "sfx.play sounds/beep.wav",
                default_keys: &[InputKey::P],
            });
    }
}

/// How many effects `sfx.play` has started; names continue from it.
#[derive(Resource, Default)]
struct EffectCount(u32);

/// `sfx.play <path>`.
fn play_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let path: String = args.parse(0, "path")?;
    let sound = SoundPlayback::effect(path);
    sound.validate().map_err(|message| args.failure(message))?;
    let name = next_free_effect_name(world);
    world.spawn((Name::new(name), sound));
    Ok(CommandOutcome::Continue)
}

/// `sfx.music <path>|off`.
fn music_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let path: String = args.parse(0, "path")?;
    stop_music(world);
    if path != "off" {
        let sound = SoundPlayback::music(path);
        sound.validate().map_err(|message| args.failure(message))?;
        world.spawn((Name::new(MUSIC_ENTITY_NAME), sound));
    }
    Ok(CommandOutcome::Continue)
}

/// Despawns the music entity, if there is one.
fn stop_music(world: &mut World) {
    let mut music = world.query_filtered::<(Entity, &Name), With<SoundPlayback>>();
    let old: Vec<Entity> = music
        .iter(world)
        .filter(|(_, name)| name.as_str() == MUSIC_ENTITY_NAME)
        .map(|(entity, _)| entity)
        .collect();
    for entity in old {
        world.despawn(entity);
    }
}

/// `sfx_<n>`: the first number whose name nobody uses yet.
fn next_free_effect_name(world: &mut World) -> String {
    let mut names = world.query::<&Name>();
    let taken: Vec<String> = names
        .iter(world)
        .map(|name| name.as_str().to_string())
        .collect();
    let mut count = world.resource_mut::<EffectCount>();
    loop {
        count.0 += 1;
        let candidate = format!("sfx_{}", count.0);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
}
