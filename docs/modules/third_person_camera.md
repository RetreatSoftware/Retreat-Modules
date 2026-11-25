# third_person_camera (built)

Rules: `crates/retreat_module_third_person_camera/src/lib.rs`. Numbers: `ShoulderConfig::default()` in
`src/config.rs`. Commands: `shoulder.follow <camera> [player]`, `shoulder.stop <camera>`, `shoulder.swap`
(key Q), `assert.shoulder_distance`.

It follows the drawn (smoothed) position every frame. Against a wall it rises over the character and
tilts down (`close_distance`, `close_lift`, `close_tilt`).
