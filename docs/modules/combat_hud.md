# combat_hud (built)

Rules and widgets: `crates/retreat_module_combat_hud/src/lib.rs`; numbers
`CombatHudConfig::default()`. Commands: `hud.scoreboard <on|off>`, `assert.hud_shown`,
`assert.hud_text`. Action: `hud_scoreboard` (Tab, held).

Not built: damage flash, sounds. Players are told apart by name (the contract's `PlayerNames`, set by
the game): the engine's HUD draws white text only, with no colors or background panels.
