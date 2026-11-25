# character_visuals (built)

Rules: `crates/retreat_module_character_visuals/src/lib.rs`. Data: `CharacterVisualsConfig` (model, clips,
colors, thresholds, smoothing); the default is the engine's plain shape with its facing.
Commands: `assert.visuals_state|facing|color` (looks follow shared state, so nothing is commanded).

Not built: hiding or animating the dead, blending between clips, turning smoothly.
