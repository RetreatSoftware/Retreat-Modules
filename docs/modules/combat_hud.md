# combat_hud (built)

Rules and widgets: `crates/retreat_module_combat_hud/src/lib.rs`; numbers
`CombatHudConfig::default()`. Commands: `hud.scoreboard <on|off>`, `assert.hud_shown`,
`assert.hud_text`. Action: `hud_scoreboard` (Tab, held).

Players are shown by the name and color of the contract's `PlayerRoster` (set by the game); text sits on
a translucent panel.

Not built: damage flash, sounds.
