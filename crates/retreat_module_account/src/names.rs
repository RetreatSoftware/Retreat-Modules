//! Login and character names: what is allowed, and the ids made from them.

use std::ops::RangeInclusive;

use retreat_gameplay_api::AccountId;

/// Why `name` cannot be a login or character name, or `None` when it can: only letters A-Z and
/// digits (so it is also a record name once lowercased), within `length`, starting with a letter.
pub fn name_problem(name: &str, length: &RangeInclusive<usize>) -> Option<String> {
    let count = name.chars().count();
    if !length.contains(&count) {
        return Some(format!(
            "a name has {} to {} letters or digits; `{name}` has {count}",
            length.start(),
            length.end()
        ));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(format!(
            "a name has only letters A-Z and digits; `{name}` has other characters"
        ));
    }
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Some(format!("a name starts with a letter; `{name}` does not"));
    }
    None
}

/// The account of a login name: the name in lowercase (`Alice` and `alice` are one account).
pub fn account_id_for_login(name: &str) -> AccountId {
    AccountId(name.to_ascii_lowercase())
}

/// The key of a character name in a world's name list: lowercase (names are unique ignoring case).
pub(crate) fn name_key(name: &str) -> String {
    name.to_ascii_lowercase()
}
