# third_person_camera

Over-the-shoulder view of the local player. Client only; it changes a `GameCamera` and no game state.

- **Commands:** `shoulder.follow <camera> <player>`, `shoulder.stop <camera>`, `shoulder.swap` (other shoulder).
- **Actions:** `shoulder_swap` Q.
- **Reads:** the followed character's pose and `LookAngles`, `PlayerRespawned`, `Health`.
- **Writes:** `GameCamera`.
- **Config (default):** pivot height 1.5 m, distance back 3.5 m, shoulder offset 0.6 m right,
  field of view 70 degrees, mouse sensitivity 0.0022 radians per count, wall margin 0.2 m (the camera
  moves in when a cast from the pivot hits geometry). While dead it stays where the player died.
- **Waits for:** C2 (mouse look, cursor capture), C3 (camera collision).
