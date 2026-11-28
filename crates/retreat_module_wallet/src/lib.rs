//! Wallet (crawler plan, phase 2): every character's safe balance. Currency is both (decided with
//! the user): gold found in dungeons is a carried item (a stack in the bag, lost on death in phase
//! 4) until it is deposited into the safe balance in the city; the balance is never lost.
//!
//! **One balance per account, in every world** (decided with the user 2026-11-28; carried gold
//! stays in each character's bag). It is saved in this module's record part `wallet` of
//! `account/<id>` (no world in the key). Every world server shares the records, so two characters
//! of one account in two worlds may deposit at the same moment: a deposit or withdrawal reads the
//! balance fresh and writes bag and balance only if the balance record is still what it read
//! (`ExtraRecords::expect_unchanged`); if another world wrote it in between, nothing moves and the
//! player is told to try again. The contract's `Wallet` on the entity `wallet_<character>`, made
//! on the authoritative machine while the character plays (private to its player), shows the
//! account's balance and is read again every [`WalletConfig::refresh_seconds`].
//!
//! Commands (player requests, routed to the server; only in a safe area within reach of a deposit
//! spot, the map's `ServiceSpots` of kind `deposit`): `wallet.deposit` moves all carried gold from
//! the bag into the balance; `wallet.withdraw <amount>` takes gold out of the balance into the bag
//! (onto gold stacks, then new stacks; refused when it does not fit). Each is one atomic save of
//! the bag and the balance (the contract's item transaction), so gold is never doubled or lost.
//! A "Gold deposit" panel (Deposit all, an amount field to withdraw) shows at a deposit spot.
//! Assertion: `assert.wallet_balance <amount>`.

use bevy::prelude::*;
use retreat_engine_api::serde_json::{Value, json};
use retreat_engine_api::{
    AppCommandExt, AppNetExt, AppUiExt, CommandArgs, CommandOutcome, CommandResult, NetLocalClient,
    NetPrivateTo, PanelAnchor, PanelDirection, RecordKey, RecordStore, RemoveNetEntity,
    RetiredEntity, StoredRecord, UiButtonEntry, UiPanel, UiPanels, UiTextField,
};
use retreat_gameplay_api::{
    AccountId, CharacterSize, ContainerKind, ExtraRecords, GameplaySystems, ItemTransaction,
    NewItems, PlayingCharacter, ServiceSpots, SoloPlayer, Wallet, account_record_owner, actor,
    add_gameplay_contract, add_to_container, apply_item_transaction, clear_item_notice,
    container_id, container_items, in_safe_area, is_authority, local_player_id, named_entity,
    playing_body, playing_characters, refused, route_request, show_item_notice, sync_item_privacy,
    ticks_from_seconds, wallet_entity_name,
};

/// This module's record part (and id).
pub const WALLET_RECORD_PART: &str = "wallet";
const WALLET_RECORD_VERSION: u32 = 1;
/// The `ServiceSpots` kind of a deposit spot.
pub const DEPOSIT_SPOT_KIND: &str = "deposit";

/// Which base is gold, and how often a shown balance is read again (placeholder; insert before
/// adding the plugin to change).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WalletConfig {
    pub gold_base: String,
    /// How often each playing character's shown balance is read again, so it shows what the
    /// account's characters in other worlds deposited or withdrew.
    pub refresh_seconds: f32,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            gold_base: "gold".to_string(),
            refresh_seconds: 0.5,
        }
    }
}

const PANEL: &str = "wallet";
const PANEL_ENTRIES: &[UiButtonEntry] = &[UiButtonEntry {
    id: "deposit",
    label: "Deposit all gold",
    command_line: "wallet.deposit",
}];
const PANEL_FIELDS: &[UiTextField] = &[UiTextField {
    id: "withdraw",
    placeholder: "Withdraw amount",
    max_chars: 9,
    submit_command: "wallet.withdraw",
    clear_on_submit: true,
}];

/// Adds wallets and the `wallet.*` commands.
pub struct WalletModulePlugin;

impl Plugin for WalletModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<WalletConfig>()
            .register_command("wallet.deposit", "wallet.deposit", deposit_command)
            .allow_client_command("wallet.deposit")
            .register_command(
                "wallet.withdraw",
                "wallet.withdraw <amount>",
                withdraw_command,
            )
            .allow_client_command("wallet.withdraw")
            .register_command(
                "assert.wallet_balance",
                "assert.wallet_balance <amount>",
                assert_balance_command,
            )
            .register_panel(UiPanel {
                id: PANEL,
                title: Some("Gold deposit"),
                anchor: PanelAnchor::TopRight,
                direction: PanelDirection::Column,
                entries: PANEL_ENTRIES,
                visible: false,
            })
            .register_text_fields(PANEL, PANEL_FIELDS)
            .add_systems(FixedUpdate, keep_wallets.in_set(GameplaySystems::Items))
            .add_systems(Update, show_panel_at_deposit);
    }
}

/// Where an account's balance is saved (every world of the account).
pub fn wallet_key(account: &AccountId) -> RecordKey {
    RecordKey::new(account_record_owner(account), WALLET_RECORD_PART)
}

fn wallet_record(balance: u64) -> Value {
    json!({ "balance": balance })
}

/// The saved balance of an account now and the record it came from (0 and `None` without
/// records or a record).
fn saved_balance(
    world: &World,
    account: &AccountId,
) -> Result<(u64, Option<StoredRecord>), String> {
    let Some(store) = world.get_resource::<RecordStore>() else {
        return Ok((0, None));
    };
    let key = wallet_key(account);
    let Some(record) = store.read(&key).map_err(|error| error.to_string())? else {
        return Ok((0, None));
    };
    record
        .check_not_newer_than(&key, WALLET_RECORD_VERSION)
        .map_err(|error| error.to_string())?;
    let balance = record
        .data
        .get("balance")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{}: no `balance` number", key.owner))?;
    Ok((balance, Some(record)))
}

/// Every playing character has its wallet entity; wallets of characters that left go; shown
/// balances are read again now and then (other worlds).
fn keep_wallets(world: &mut World, mut ticks: Local<u64>) {
    if !is_authority(world) {
        return;
    }
    let timestep = world.resource::<Time<Fixed>>().timestep();
    let every = ticks_from_seconds(world.resource::<WalletConfig>().refresh_seconds, timestep);
    *ticks += 1;
    let refresh = (*ticks).is_multiple_of(u64::from(every.max(1)));
    let playing = playing_characters(world);
    let stale: Vec<Entity> = world
        .query_filtered::<(Entity, &Wallet), (Without<RetiredEntity>, Without<RemoveNetEntity>)>()
        .iter(world)
        .filter(|(_, wallet)| !playing.iter().any(|(_, p)| p.id == wallet.character))
        .map(|(entity, _)| entity)
        .collect();
    for entity in stale {
        world.entity_mut(entity).insert(RemoveNetEntity);
    }
    for (player, character) in playing {
        let name = wallet_entity_name(&character.id);
        if let Some(entity) = named_entity(world, &name) {
            if refresh {
                show_saved_balance(world, entity, &character.account);
            }
            continue;
        }
        match saved_balance(world, &character.account) {
            Ok((balance, _)) => {
                world.spawn((
                    Name::new(name),
                    Wallet {
                        character: character.id.clone(),
                        balance,
                    },
                    NetPrivateTo {
                        client_id: player.0,
                    },
                ));
            }
            Err(problem) => error!("wallet of {} cannot be read: {problem}", character.id.0),
        }
    }
    sync_item_privacy(world);
}

/// Sets the shown balance of wallet entity `entity` to `account`'s saved balance.
fn show_saved_balance(world: &mut World, entity: Entity, account: &AccountId) {
    match saved_balance(world, account) {
        Ok((balance, _)) => {
            if world.get::<Wallet>(entity).map(|wallet| wallet.balance) != Some(balance) {
                world
                    .get_mut::<Wallet>(entity)
                    .expect("a wallet entity")
                    .balance = balance;
            }
        }
        Err(problem) => error!("the balance of {} cannot be read: {problem}", account.0),
    }
}

/// The actor's body, character and wallet entity when it stands at a deposit spot in a safe area.
fn at_deposit(
    world: &mut World,
    args: &CommandArgs,
) -> Result<
    (
        Entity,
        PlayingCharacter,
        Entity,
        retreat_gameplay_api::PlayerId,
    ),
    String,
> {
    let player = actor(world, args).map_err(|error| error.to_string())?;
    let (body, playing) = playing_body(world, player).ok_or("you are not playing a character")?;
    let feet = world
        .get::<Transform>(body)
        .map(|transform| {
            world
                .resource::<CharacterSize>()
                .feet(transform.translation)
        })
        .ok_or("you have no place in the world")?;
    if !in_safe_area(world, body)
        || !world
            .resource::<ServiceSpots>()
            .near(DEPOSIT_SPOT_KIND, feet)
    {
        return Err("walk up to the gold deposit in the city".to_string());
    }
    let wallet = named_entity(world, &wallet_entity_name(&playing.id))
        .ok_or("your balance is not ready yet")?;
    Ok((body, playing, wallet, player))
}

/// Answers a request's result: a refusal becomes the player's notice.
fn answer(world: &mut World, args: &CommandArgs, result: Result<(), String>) -> CommandResult {
    match result {
        Ok(()) => {
            let player = actor(world, args)?;
            if let Some((body, _)) = playing_body(world, player) {
                clear_item_notice(world, body);
            }
            Ok(CommandOutcome::Continue)
        }
        Err(why) => {
            let player = actor(world, args)?;
            // A refusal is the player's notice (not an error); without a character there is no one
            // to tell, which a script should hear about.
            let Some((body, _)) = playing_body(world, player) else {
                return refused(world, args, &why);
            };
            show_item_notice(world, body, why);
            Ok(CommandOutcome::Continue)
        }
    }
}

/// `wallet.deposit`: all carried gold into the balance.
fn deposit_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    if !route_request(world, "wallet.deposit".to_string()) {
        return Ok(CommandOutcome::Continue);
    }
    let result = deposit(world, args);
    answer(world, args, result)
}

fn deposit(world: &mut World, args: &CommandArgs) -> Result<(), String> {
    let (_, playing, wallet, player) = at_deposit(world, args)?;
    let gold = world.resource::<WalletConfig>().gold_base.clone();
    let stacks: Vec<_> = container_items(world, &container_id(ContainerKind::Bag, &playing.id))
        .into_iter()
        .filter(|placed| placed.item.base == gold)
        .collect();
    let amount: u64 = stacks
        .iter()
        .map(|placed| u64::from(placed.item.quantity))
        .sum();
    if amount == 0 {
        return Err("you carry no gold".to_string());
    }
    // The balance as saved now (another world may have changed it since it was shown).
    let (now, seen) = saved_balance(world, &playing.account)?;
    let balance = now + amount;
    let result = apply_item_transaction(
        world,
        player,
        ItemTransaction {
            put: Vec::new(),
            remove: stacks.iter().map(|placed| placed.item.uid).collect(),
            records: balance_change(&playing.account, seen, balance),
        },
    );
    if result.is_err() {
        show_saved_balance(world, wallet, &playing.account);
    }
    result?;
    world
        .get_mut::<Wallet>(wallet)
        .expect("a wallet entity")
        .balance = balance;
    Ok(())
}

/// `wallet.withdraw <amount>`: gold from the balance into the bag.
fn withdraw_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let amount: u32 = args.parse(0, "amount")?;
    if !route_request(world, format!("wallet.withdraw {amount}")) {
        return Ok(CommandOutcome::Continue);
    }
    let result = withdraw(world, args, amount);
    answer(world, args, result)
}

fn withdraw(world: &mut World, args: &CommandArgs, amount: u32) -> Result<(), String> {
    if amount == 0 {
        return Err("withdraw at least 1 gold".to_string());
    }
    let (_, playing, wallet, player) = at_deposit(world, args)?;
    let (balance, seen) = saved_balance(world, &playing.account)?;
    if u64::from(amount) > balance {
        return Err(format!("your balance is only {balance} gold"));
    }
    let gold = world.resource::<WalletConfig>().gold_base.clone();
    let left = balance - u64::from(amount);
    let result = add_to_container(
        world,
        player,
        &container_id(ContainerKind::Bag, &playing.id),
        NewItems {
            unique: Vec::new(),
            stack: Some((gold, amount)),
        },
        balance_change(&playing.account, seen, left),
    );
    if result.is_err() {
        show_saved_balance(world, wallet, &playing.account);
    }
    result?;
    world
        .get_mut::<Wallet>(wallet)
        .expect("a wallet entity")
        .balance = left;
    Ok(())
}

/// The account's new balance, written only if its record is still `seen`.
fn balance_change(account: &AccountId, seen: Option<StoredRecord>, balance: u64) -> ExtraRecords {
    ExtraRecords {
        put: vec![(
            wallet_key(account),
            WALLET_RECORD_VERSION,
            wallet_record(balance),
        )],
        expect_unchanged: vec![(wallet_key(account), seen)],
    }
}

/// The wallet of this machine's player.
fn own_wallet(world: &mut World) -> Option<(Entity, Option<Wallet>)> {
    let player = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )?;
    let (body, playing) = playing_body(world, player)?;
    let wallet = named_entity(world, &wallet_entity_name(&playing.id))
        .and_then(|entity| world.get::<Wallet>(entity).cloned());
    Some((body, wallet))
}

/// `assert.wallet_balance <amount>` (this machine's player).
fn assert_balance_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: u64 = args.parse(0, "amount")?;
    let Some((_, Some(wallet))) = own_wallet(world) else {
        return Err(
            args.failure("this machine's player has no balance (not playing, or not yet sent)")
        );
    };
    if wallet.balance == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "the balance is {}, expected {expected}",
            wallet.balance
        )))
    }
}

/// Shows the panel while this machine's character stands at a deposit spot.
fn show_panel_at_deposit(world: &mut World) {
    let visible = own_wallet(world).is_some_and(|(body, _)| {
        world
            .get::<Transform>(body)
            .map(|transform| {
                world
                    .resource::<CharacterSize>()
                    .feet(transform.translation)
            })
            .is_some_and(|feet| {
                world
                    .resource::<ServiceSpots>()
                    .near(DEPOSIT_SPOT_KIND, feet)
            })
    });
    if world.resource::<UiPanels>().is_visible(PANEL) != visible {
        world
            .resource_mut::<UiPanels>()
            .set_visible(PANEL, visible)
            .expect("registered");
    }
}
