# projectile_launcher

Rockets with blast damage. Predicted on the shooter's client, decided by the server.

- **Commands:** `launcher.fire <yaw> <pitch>`, `launcher.reload` (client allowed),
  `assert.launcher_ammo <player> <n>`, `assert.launcher_projectiles <n>`.
- **Actions:** `launcher_fire` MouseRight, `launcher_reload` T (the rifle takes R).
- **Reads:** `Health`, `MatchState`, `PlayerRespawned` (refill), `MatchRestarted` (remove rockets).
- **Writes:** rocket entities, `DamageRequested` (`Projectile` for the direct hit, `Blast` for the area).
- **Config (default):**

| Key | Value |
|---|---|
| direct hit damage | 90 (no blast damage added to the same target) |
| blast radius | 4 m |
| blast damage | 70 at the center to 20 at the edge, linear; walls block it |
| self-damage | half |
| knockback | none |
| speed / lifetime | 25 m/s, no gravity / 4 s |
| fire interval | 0.9 s |
| magazine | 4, unlimited reserve |
| reload | 2 s, automatic when empty |

- **Waits for:** C1, C2, C3, C8.
