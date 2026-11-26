# combat_hud (step 3 part built)

Built: crosshair, health, every carried weapon with its ammo, hit marker, respawn countdown
(`crates/retreat_module_combat_hud/src/lib.rs`; numbers `CombatHudConfig::default()`). Command:
`assert.hud_shown`; text with the engine's `assert.ui_text`.

Still to do (plan step 5): match timer, scoreboard (`hud.scoreboard`, Tab), result, kill feed, damage
flash. Each element is hidden when its data is absent.
