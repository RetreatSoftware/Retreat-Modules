//! Accounts and characters (crawler plan, phase 1): a dev login by name (no password, until
//! platform accounts exist), one account with several characters, each bound to the world (the
//! server) it was made on; create, list, play, log out and delete characters. Saved in the
//! server's records (`RecordStore`), never on a client.
//!
//! **Login.** On a network client the login name is the identity token of the client's hello
//! (`NetClientIdentity`, the client waits for it): `account.login <name>` sets it and the client
//! joins. The server's identity check ([`DevNameLogin`], inserted by this plugin) lets the name in as
//! player id `dev:<lowercase name>`; one connection per player id is the engine's rule, so logging
//! in again elsewhere takes over. Without a network `account.login <name>` logs in the solo player
//! (id 0) directly.
//!
//! **Where things are decided.** Every `account.*` command that changes an account follows one
//! rule ([`commands::route`]): typed or pressed on a network client it is sent to the server and
//! does nothing else; a client's own prediction of it does nothing; on the server (or without a
//! network) it acts, for the player who sent it. Refusals (a taken name, too many characters) are
//! shown to that player as the session's notice.
//!
//! **Shared state.** Each logged-in player has a session entity `account/<player>` with
//! [`AccountSession`] (account, world, characters, the one being played, a notice). Every client
//! receives every session (the engine has no per-client state yet; the lists hold names only).
//!
//! **Records** (one part `account` per owner): `account/<account>` holds the character list and the
//! next character number; `character/<id>` holds name, account and world; `world/<world>` holds the
//! character names taken in that world (names are unique per world, ignoring case). Creating and
//! deleting a character are each one atomic batch over those records; deleting also removes every
//! other module's part of the character.
//!
//! **Playing.** `account.play <character>` writes `CharacterEntered` (the spawn module places the
//! body where the character was); `account.logout` writes `CharacterLeft` and returns to the list.
//! A disconnect ends the session (the spawn module saves the place itself, from `NetClientLeft`);
//! a takeover (`NetClientRejoined` in the same tick) moves the session, still playing, to the new
//! connection.
//!
//! **Screens** (this machine's player; pure [`account_screen`]): Login (name field), Joining,
//! Characters (list, new-name field, Previous/Next/Play/Delete, Delete asks for a second press),
//! Playing (Log out at the top right). Text and lines are engine `UiHud` widgets; the list has one
//! line per allowed character ([`AccountConfig::max_characters`]).
//!
//! Numbers: [`AccountConfig`].

mod commands;
mod config;
mod identity;
mod names;
mod records;
mod screen;
mod session;

use bevy::prelude::*;
use retreat_engine_api::{
    AppCommandExt, AppNetExt, AppNetStateExt, NetClientIdentity, NetIdentity,
};
use retreat_gameplay_api::{GameplaySystems, add_gameplay_contract};

pub use config::AccountConfig;
pub use identity::{DevNameLogin, account_of_player_id, player_id_for_account};
pub use names::{account_id_for_login, name_problem};
pub use screen::{AccountScreen, account_screen};
pub use session::{AccountSession, CharacterSummary, session_entity_name};

/// Adds accounts, characters, their screens and the `account.*` commands. Insert an
/// [`AccountConfig`] first to change the world name or the limits.
pub struct AccountModulePlugin;

impl Plugin for AccountModulePlugin {
    fn build(&self, app: &mut App) {
        add_gameplay_contract(app);
        app.init_resource::<AccountConfig>();
        if let Err(problem) = app.world().resource::<AccountConfig>().validate() {
            panic!("invalid AccountConfig: {problem}");
        }
        let config = app.world().resource::<AccountConfig>().clone();
        // The server lets a client in by its login name; a client says hello only with one.
        app.insert_resource(NetIdentity::new(DevNameLogin {
            name_length: config.name_length.clone(),
        }));
        app.insert_resource(NetClientIdentity {
            wait_for_token: true,
            token: None,
        });
        app.register_net_state::<AccountSession>()
            .init_resource::<screen::AccountPicker>()
            .register_command(
                "account.login",
                "account.login <name>",
                commands::login_command,
            )
            .register_command(
                "account.create",
                "account.create <character name>",
                commands::create_command,
            )
            .allow_client_command("account.create")
            .register_command(
                "account.play",
                "account.play <character id>",
                commands::play_command,
            )
            .allow_client_command("account.play")
            .register_command("account.logout", "account.logout", commands::logout_command)
            .allow_client_command("account.logout")
            .register_command(
                "account.delete",
                "account.delete <character id>",
                commands::delete_command,
            )
            .allow_client_command("account.delete")
            .register_command(
                "account.pick_next",
                "account.pick_next",
                screen::pick_next_command,
            )
            .register_command(
                "account.pick_previous",
                "account.pick_previous",
                screen::pick_previous_command,
            )
            .register_command(
                "account.play_picked",
                "account.play_picked",
                screen::play_picked_command,
            )
            .register_command(
                "account.delete_picked",
                "account.delete_picked",
                screen::delete_picked_command,
            )
            .register_command(
                "assert.account_characters",
                "assert.account_characters <player> [character names...]",
                commands::assert_characters_command,
            )
            .register_command(
                "assert.account_playing",
                "assert.account_playing <player> <character name|nobody>",
                commands::assert_playing_command,
            )
            .register_command(
                "assert.account_screen",
                "assert.account_screen <login|joining|characters|playing>",
                screen::assert_screen_command,
            );
        screen::register_screens(app, &config);
        app.add_systems(
            FixedUpdate,
            (
                session::move_sessions_of_rejoined_players,
                session::open_sessions_of_joined_clients,
                session::close_sessions_of_left_clients,
            )
                .chain()
                // Before the spawn module reads this tick's `CharacterEntered` (it runs in `Act`).
                .before(GameplaySystems::Act),
        )
        .add_systems(Update, screen::show_screen);
    }
}
