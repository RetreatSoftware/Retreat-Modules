# character_movement (built)

Rules: `crates/retreat_module_character_movement/src/lib.rs`; the one-tick rule is `step_movement` in
`src/motion.rs`. Numbers: `MovementConfig::default()` in `src/config.rs`.
Commands: `movement.input` (client allowed), `movement.look`, `assert.movement_grounded|speed|near|look`.
Actions: WASD, ShiftLeft sprint, Space jump, ControlLeft dash, click captures the cursor, Escape releases it.

Stairs and slopes come from the engine (`CharacterStepping`: step height 0.35 m, 45 degrees); a respawn
teleports the character. Not built: gamepad.
