# character_visuals

What a character looks like: capsule or fox, one color per player, animation. Client only.

- **Commands:** `visuals.model <capsule|fox>`, `visuals.color <player> <r> <g> <b>`.
- **Actions:** none.
- **Reads:** `PlayerId`, `Health` (hide the dead), character velocity, `LookAngles`.
- **Writes:** child visuals, `AnimationPlayback` (fox: idle, walk, run from speed; its own pure rule,
  not the `fox_walk` module's).
- **Config (default):** model capsule; colors by player id modulo 8: red, blue, green, yellow, purple,
  orange, cyan, white.
- **Waits for:** C5, C6.
