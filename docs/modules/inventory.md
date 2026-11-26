# inventory (built, plan step L1)

Rules: `crates/retreat_module_inventory/src/lib.rs`; contract: `retreat_gameplay_api/src/item.rs`.
Numbers: `InventoryConfig` (slots, starting loadout) and the game's `ItemCatalog`. Commands:
`inventory.grant <player> <item> <count>` (server and scripts), `inventory.use <item>` (client
allowed), `assert.inventory <player> <item> <count>`. Action: `inventory_use` (E).

Not built: dropping or choosing items by hand, saving, sounds. Pickups and death drops are `loot`
(L2).
