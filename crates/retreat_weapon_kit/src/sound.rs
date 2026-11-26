//! Sounds every weapon plays the same way.

use bevy::prelude::*;
use retreat_engine_api::{EffectTime, SoundEvents};
use retreat_gameplay_api::{PlayerId, SoundCue, WeaponKind, WeaponStatus};

/// This machine's player hears its own weapon reload. Call it every tick: the id comes from the
/// shots fired so far (a new reload needs a new shot), so each reload is heard once.
pub fn play_reload(
    events: &mut SoundEvents,
    cue: Option<&SoundCue>,
    player: PlayerId,
    weapon: &WeaponStatus,
    shots: u32,
) {
    let Some(cue) = cue else {
        return;
    };
    if weapon.reload_ticks_left == 0 {
        return;
    }
    let kind = match weapon.kind {
        WeaponKind::Rifle => "rifle",
        WeaponKind::Launcher => "launcher",
    };
    let id = format!("{kind}/{}/reload/{shots}", player.0);
    events.play(cue.event(id, None, EffectTime::Present));
}
