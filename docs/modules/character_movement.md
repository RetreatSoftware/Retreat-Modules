# character_movement (built)

Rules: `crates/retreat_module_character_movement/src/lib.rs`; the one-tick rule is `step_movement` in
`src/motion.rs`. Numbers: `MovementConfig::default()` in `src/config.rs`.
Commands: `movement.input` (client allowed), `movement.look`, `assert.movement_grounded|speed|near`.
Actions: WASD, ShiftLeft sprint, Space jump, ControlLeft dash, click captures the cursor, Escape releases it.

Not built: slopes and a step-up rule (low risers work through the capsule's rounded bottom), gamepad,
teleport on respawn (needs an engine capability).
