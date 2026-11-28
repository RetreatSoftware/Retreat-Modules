//! Item rolls (crawler plan, phase 2): makes grid item instances from the game's
//! [`GridItemCatalog`]: a base, a rarity tier picked by weight, as many random affixes from the
//! base's pool as the rarity allows, values lifted by the item level, and a level requirement.
//! Rolls draw from `SimulationRng` (deterministic under a script). Every instance gets a new unique
//! uid from the contract's saved counter.
//!
//! New items go into the character's bag (the contract container `bag/<character>`, owned by the
//! bag module) in one atomic save; when they do not all fit, nothing is given and the player is told.
//! Without a bag nothing can be given.
//!
//! Commands (player requests, routed to the server like every crawler request):
//! `items.grant <base> <rarity> <item level>` (a chosen item), `items.grant_test <count>` (random
//! items at random item levels up to [`ItemRollsConfig::test_max_item_level`]),
//! `items.grant_gold <amount>`. They are test tools until dungeons and loot exist (phase 3):
//! clients may send them only when [`ItemRollsConfig::test_commands`] is on, which also shows a
//! "Test items" panel with buttons while playing.
//!
//! Numbers and the level-requirement rule (requirement = item level) are placeholders.

mod roll;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppNetExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult, NetLocalClient,
    PanelAnchor, PanelDirection, SimulationRng, UiButtonEntry, UiPanel, UiPanels,
};
use retreat_gameplay_api::{
    ContainerKind, GridItemCatalog, NewItems, SoloPlayer, actor, add_gameplay_contract,
    add_to_container, container_id, local_player_id, playing_body, refused, route_request,
    show_item_notice,
};

pub use roll::{Dice, pick_rarity, roll_item, roll_random};

/// Placeholder numbers and switches of item rolls (insert before adding the plugin to change).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ItemRollsConfig {
    /// Test commands from clients and the test panel (off by default; a testbed turns it on).
    pub test_commands: bool,
    /// The highest item level `items.grant_test` rolls (each item 1 to this).
    pub test_max_item_level: u32,
    /// The base `items.grant_gold` gives.
    pub gold_base: String,
}

impl Default for ItemRollsConfig {
    fn default() -> Self {
        Self {
            test_commands: false,
            test_max_item_level: 10,
            gold_base: "gold".to_string(),
        }
    }
}

const TEST_PANEL: &str = "item_rolls_test";
const TEST_ENTRIES: &[UiButtonEntry] = &[
    UiButtonEntry {
        id: "items",
        label: "Get test items",
        command_line: "items.grant_test 3",
    },
    UiButtonEntry {
        id: "gold",
        label: "Get 250 gold",
        command_line: "items.grant_gold 250",
    },
];

/// Adds item rolls and the `items.*` commands.
pub struct ItemRollsModulePlugin;

impl Plugin for ItemRollsModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<ItemRollsConfig>();
        let config = app.world().resource::<ItemRollsConfig>().clone();
        app.register_command(
            "items.grant",
            "items.grant <base> <rarity> <item level>",
            grant_command,
        )
        .register_command(
            "items.grant_test",
            "items.grant_test <count>",
            grant_test_command,
        )
        .register_command(
            "items.grant_gold",
            "items.grant_gold <amount>",
            grant_gold_command,
        )
        .register_panel(UiPanel {
            id: TEST_PANEL,
            title: Some("Test items"),
            anchor: PanelAnchor::TopLeft,
            direction: PanelDirection::Column,
            entries: TEST_ENTRIES,
            visible: false,
        })
        .add_systems(Update, show_test_panel_while_playing);
        if config.test_commands {
            app.allow_client_command("items.grant")
                .allow_client_command("items.grant_test")
                .allow_client_command("items.grant_gold");
        }
    }
}

/// `SimulationRng` as [`Dice`].
struct SimulationDice<'a>(&'a mut SimulationRng);

impl Dice for SimulationDice<'_> {
    fn below(&mut self, n: u64) -> u64 {
        self.0.0.next_u64() % n
    }
}

/// Puts `new` into the actor's bag, or tells the player why not.
fn give(world: &mut World, args: &CommandArgs, new: NewItems) -> CommandResult {
    let player = actor(world, args)?;
    let Some((body, playing)) = playing_body(world, player) else {
        return refused(world, args, "you are not playing a character");
    };
    let bag = container_id(ContainerKind::Bag, &playing.id);
    match add_to_container(world, player, &bag, new, Vec::new()) {
        Ok(_) => Ok(CommandOutcome::Continue),
        Err(why) => {
            show_item_notice(world, body, why.clone());
            refused(world, args, &why)
        }
    }
}

/// `items.grant <base> <rarity> <item level>`.
fn grant_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(3)?;
    let base: String = args.parse(0, "base")?;
    let rarity: String = args.parse(1, "rarity")?;
    let level: u32 = args.parse(2, "item level")?;
    if !route_request(world, format!("items.grant {base} {rarity} {level}")) {
        return Ok(CommandOutcome::Continue);
    }
    let catalog = world.resource::<GridItemCatalog>().clone();
    let base = catalog.base(&base).ok_or_else(|| {
        args.failure(format!(
            "unknown item base `{base}`; known: {:?}",
            catalog
                .bases
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>()
        ))
    })?;
    let rarity = catalog.rarity(&rarity).ok_or_else(|| {
        args.failure(format!(
            "unknown rarity `{rarity}`; known: {:?}",
            catalog
                .rarities
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>()
        ))
    })?;
    let item = {
        let mut rng = world.resource_mut::<SimulationRng>();
        roll_item(&catalog, base, rarity, level, &mut SimulationDice(&mut rng))
    };
    give(
        world,
        args,
        NewItems {
            unique: vec![item],
            stack: None,
        },
    )
}

/// `items.grant_test <count>`: random items at random item levels.
fn grant_test_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let count: u32 = args.parse(0, "count")?;
    if !(1..=20).contains(&count) {
        return Err(args.failure("count must be 1 to 20"));
    }
    if !route_request(world, format!("items.grant_test {count}")) {
        return Ok(CommandOutcome::Continue);
    }
    let catalog = world.resource::<GridItemCatalog>().clone();
    let max_level = world
        .resource::<ItemRollsConfig>()
        .test_max_item_level
        .max(1);
    let mut items = Vec::new();
    {
        let mut rng = world.resource_mut::<SimulationRng>();
        let mut dice = SimulationDice(&mut rng);
        for _ in 0..count {
            let level = 1 + dice.below(u64::from(max_level)) as u32;
            let Some(item) = roll_random(&catalog, level, &mut dice) else {
                return Err(args.failure("the game's item data has no rollable base"));
            };
            items.push(item);
        }
    }
    give(
        world,
        args,
        NewItems {
            unique: items,
            stack: None,
        },
    )
}

/// `items.grant_gold <amount>`.
fn grant_gold_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let amount: u32 = args.parse(0, "amount")?;
    if amount == 0 {
        return Err(args.failure("amount must be at least 1"));
    }
    if !route_request(world, format!("items.grant_gold {amount}")) {
        return Ok(CommandOutcome::Continue);
    }
    let gold = world.resource::<ItemRollsConfig>().gold_base.clone();
    if world.resource::<GridItemCatalog>().base(&gold).is_none() {
        return Err(args.failure(format!("the game's item data has no gold base `{gold}`")));
    }
    give(
        world,
        args,
        NewItems {
            unique: Vec::new(),
            stack: Some((gold, amount)),
        },
    )
}

/// Shows the test panel while this machine's player plays a character (and tests are on).
fn show_test_panel_while_playing(world: &mut World) {
    let wanted = world.resource::<ItemRollsConfig>().test_commands
        && local_player_id(
            *world.resource::<NetLocalClient>(),
            *world.resource::<SoloPlayer>(),
        )
        .is_some_and(|player| playing_body(world, player).is_some());
    if world.resource::<UiPanels>().is_visible(TEST_PANEL) != wanted {
        world
            .resource_mut::<UiPanels>()
            .set_visible(TEST_PANEL, wanted)
            .expect("registered");
    }
}
