# loot (built, plan step L2)

Rules: `crates/retreat_module_loot/src/lib.rs`. Numbers and contents: the game's `LootConfig`; the
spots are the map's (`retreat_module_arena`, `PICKUP_SPOTS`). Commands: `assert.loot_available
<spot> <true|false>`, `assert.loot_drops <count>`.

Looks are turning, bobbing, glowing boxes, sized and colored by kind (`PickupLook`, `motion`).
Each kind (and the drops) has a sound; the combat HUD names what its player took
(`hud_pickup<n>`, from the contract's `LastPickup`). Not built: prediction of taking, models.
