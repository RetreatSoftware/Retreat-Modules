//! The server's dev login: a client's hello token is its login name.

use std::ops::RangeInclusive;

use retreat_engine_api::{NetIdentityCheck, NetIdentityDecision, NetIdentityRequest};
use retreat_gameplay_api::AccountId;

use crate::names::{account_id_for_login, name_problem};

/// The prefix of a dev login's player id.
const DEV_PLAYER_PREFIX: &str = "dev:";

/// Lets a client in when its token is a valid login name, as player id `dev:<account>`. No
/// password: a development login until platform accounts exist (their check replaces this one).
#[derive(Debug, Clone)]
pub struct DevNameLogin {
    pub name_length: RangeInclusive<usize>,
}

impl NetIdentityCheck for DevNameLogin {
    fn check(&self, request: NetIdentityRequest) -> NetIdentityDecision {
        let Some(name) = request.token else {
            return NetIdentityDecision::Deny {
                reason: "no login name in the hello (the client must log in with a name)"
                    .to_string(),
            };
        };
        match name_problem(name, &self.name_length) {
            Some(problem) => NetIdentityDecision::Deny { reason: problem },
            None => NetIdentityDecision::Allow {
                player_id: player_id_for_account(&account_id_for_login(name)),
            },
        }
    }
}

/// The player id of an account's connection: `dev:<account>`.
pub fn player_id_for_account(account: &AccountId) -> String {
    format!("{DEV_PLAYER_PREFIX}{}", account.0)
}

/// The account of a player id made by [`DevNameLogin`].
pub fn account_of_player_id(player_id: &str) -> Option<AccountId> {
    player_id
        .strip_prefix(DEV_PLAYER_PREFIX)
        .map(|account| AccountId(account.to_string()))
}
