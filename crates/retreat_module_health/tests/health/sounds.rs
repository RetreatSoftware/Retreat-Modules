//! Death and respawn are heard.

use crate::harness::*;
use retreat_engine_api::SoundEvents;
use retreat_gameplay_api::SoundCue;
use retreat_module_health::HealthConfig;

fn heard(app: &mut bevy::prelude::App) -> Vec<String> {
    let mut ids: Vec<String> =
        std::mem::take(&mut app.world_mut().resource_mut::<SoundEvents>().requests)
            .into_iter()
            .map(|event| event.id)
            .collect();
    ids.dedup();
    ids
}

#[test]
fn each_death_and_each_respawn_has_its_own_sound_id() {
    let mut app = app_with(HealthConfig {
        death_sound: Some(SoundCue::new("sounds/death.wav", 0.5)),
        respawn_sound: Some(SoundCue::new("sounds/respawn.wav", 0.5)),
        ..HealthConfig::default()
    });
    ok(&mut app, "health.add_dummy 1");
    ticks(&mut app, PROTECTION_TICKS + 1);
    assert!(heard(&mut app).is_empty(), "the first life starts silently");
    for life in 0..2 {
        ok(&mut app, "health.kill 1");
        ticks(&mut app, 3);
        assert_eq!(heard(&mut app), [format!("death/1/{life}")]);
        ticks(&mut app, RESPAWN_TICKS);
        let ids = heard(&mut app);
        assert!(ids.contains(&format!("respawn/1/{}", life + 1)), "{ids:?}");
        ticks(&mut app, PROTECTION_TICKS + 1);
        heard(&mut app);
    }
}
