# character_movement

Walk, sprint, jump, dash for every `PlayerId` character.

- **Commands:** `movement.input <right> <forward> <yaw> <pitch> <sprint 0|1>` (one per tick, client
  allowed; axes -1..1, angles in radians), `movement.jump`, `movement.dash` (client allowed),
  `movement.look <yaw> <pitch>` (scripts), `assert.movement_speed <player> <speed>`,
  `assert.movement_grounded <player>`.
- **Actions:** `movement_forward` W, `movement_back` S, `movement_left` A, `movement_right` D,
  `movement_sprint` ShiftLeft (all held), `movement_jump` Space, `movement_dash` ControlLeft.
  Mouse motion writes yaw and pitch (C2).
- **Reads:** `Health` (the dead do not move; absent means alive), `PlayerRespawned` (teleport, zero velocity).
- **Writes:** character velocity and position, `LookAngles`.
- **Config (default):**

| Key | Value |
|---|---|
| walk speed | 6 m/s |
| sprint speed | 9 m/s |
| ground acceleration / friction | 10 / 6 (per second), stop speed 1 m/s |
| air acceleration / wish speed cap | 10 / 1 m/s |
| gravity | 19.6 m/s2 |
| jump apex | 1.5 m (jump speed 7.67 m/s) |
| max fall speed | 25 m/s |
| dash speed / duration | 18 m/s / 0.15 s (2.7 m), along the move input, else the view |
| dash cooldown | 3 s; one dash per time airborne |
| step height | 0.15 m (the arena stairs rise 0.1 m) |
| max walkable slope | 45 degrees |
| max pitch | 85 degrees |

- **Character size:** `CHARACTER_RADIUS`, `CHARACTER_HEIGHT` from the contract.
- **Waits for:** C1, C2, C3, C4.
