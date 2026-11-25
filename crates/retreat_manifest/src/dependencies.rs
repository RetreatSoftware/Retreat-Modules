//! The dependency rule: modules never depend on each other (or on engine internals, or on a
//! physics library). They share types only through contract crates.

use crate::problem::Problem;

/// Everything a module's `Cargo.toml` may depend on, in any dependency table.
pub const MODULE_DEPENDENCIES: &[&str] = &[
    "bevy",
    "retreat_engine_api",
    "retreat_gameplay_api",
    "retreat_rng",
];

/// Everything a gameplay contract crate may depend on.
pub const CONTRACT_DEPENDENCIES: &[&str] = &["bevy", "retreat_engine_api"];

/// The contract crates of this repository (`crates/<name>`).
pub const CONTRACT_CRATES: &[&str] = &["retreat_gameplay_api"];

const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// One problem per dependency of `cargo_toml` that is not in `allowed`. A renamed dependency is
/// checked by its real `package` name. Text that is not TOML has no dependencies to check (the
/// manifest parser reports it).
pub fn check_dependencies(who: &str, cargo_toml: &str, allowed: &[&str]) -> Vec<Problem> {
    let Ok(document) = cargo_toml.parse::<toml::Table>() else {
        return Vec::new();
    };
    // The top-level tables plus those under every `[target.'...']`.
    let mut scopes: Vec<&toml::Table> = vec![&document];
    if let Some(targets) = document.get("target").and_then(|t| t.as_table()) {
        scopes.extend(targets.values().filter_map(|t| t.as_table()));
    }
    let mut problems = Vec::new();
    for scope in scopes {
        for key in DEPENDENCY_TABLES {
            let Some(table) = scope.get(key).and_then(|t| t.as_table()) else {
                continue;
            };
            for (name, spec) in table {
                let real = spec.get("package").and_then(|p| p.as_str()).unwrap_or(name);
                if !allowed.contains(&real) {
                    problems.push(Problem::new(
                        who,
                        format!(
                            "depends on `{real}` ([{key}]), which is not allowed; allowed: {}. Modules never depend on each other: put shared types in a contract crate",
                            allowed.join(", ")
                        ),
                    ));
                }
            }
        }
    }
    problems
}
