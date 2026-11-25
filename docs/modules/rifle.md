# rifle

Hitscan rifle. The server decides hits, rewound to what the shooter saw.

- **Commands:** `rifle.fire <yaw> <pitch>`, `rifle.reload` (client allowed), `assert.rifle_ammo <player> <n>`.
- **Actions:** `rifle_fire` MouseLeft (held: automatic fire), `rifle_reload` R.
- **Reads:** `Health`, `MatchState`, `PlayerRespawned` (refill), `LookAngles`.
- **Writes:** `DamageRequested` (cause `Hitscan`), its ammo state, `SoundPlayback` effects.
- **Config (default):**

| Key | Value |
|---|---|
| damage per hit | 20 (5 hits kill) |
| fire interval | 0.15 s |
| magazine | 30, unlimited reserve |
| reload | 1.5 s, automatic when empty |
| range | 100 m |
| spread, falloff, headshots | none |

- The ray starts at the shooter's eyes (`CHARACTER_EYE_HEIGHT`); the client aims it at what the screen
  center points at. The server enforces fire rate and ammo.
- **Waits for:** C1, C2, C3, C7, C9 (hit marker and ammo text are drawn by `combat_hud`).
