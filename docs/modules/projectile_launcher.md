# projectile_launcher (built)

Rockets with blast damage. Predicted on the shooter's client, decided by the server. Rules:
`crates/retreat_module_projectile_launcher/src/lib.rs`. Numbers: `LauncherConfig::default()` in
`src/config.rs`.

- **Commands:** `launcher.fire <yaw> <pitch>`, `launcher.reload` (client allowed),
  `assert.launcher_ammo`, `assert.launcher_hits`, `assert.launcher_rockets <flying>`.
- **Actions:** `launcher_fire` (right mouse, held), `launcher_reload` (R, together with the rifle).
- A second fire button instead of a weapon switch (decided 2026-11-26): no shared "selected weapon"
  state and no third module; the HUD lists both weapons.

A match restart removes the rockets in flight.

Not built: sounds, knockback. No lag compensation: rockets hit
bodies where they are, so shooters lead moving targets.
