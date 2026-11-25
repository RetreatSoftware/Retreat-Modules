# third_person_camera (built)

Rules: `crates/retreat_module_third_person_camera/src/lib.rs`. Numbers: `ShoulderConfig::default()` in
`src/config.rs`. Commands: `shoulder.follow <camera> [player]`, `shoulder.stop <camera>`, `shoulder.swap`
(key Q), `assert.shoulder_distance`.

Not built: smoothing between fixed ticks. Against a wall the camera comes close to the character.
