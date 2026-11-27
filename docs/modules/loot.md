# loot (built, plan step L2)

Rules: `crates/retreat_module_loot/src/lib.rs`. Numbers and contents: the game's `LootConfig`; the
spots are the map's (`retreat_module_arena`, `PICKUP_SPOTS`). Commands: `assert.loot_available
<spot> <true|false>`, `assert.loot_drops <count>`.

Looks are turning, bobbing, glowing boxes, sized and colored by kind (`PickupLook`, `motion`).
Not built: sounds (L3), prediction of taking, models.
