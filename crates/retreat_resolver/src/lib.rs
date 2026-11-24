//! Resolver: `resolve(selection, manifests, engine_version) -> Plan | ResolveError`.
//!
//! Pure and Forge-free. Input: the module ids a game wants, every available manifest and the
//! `retreat_engine_api` version the game builds against. Output: an ordered build plan, or every
//! problem in plain language (never only the first). Checked: unknown or repeated module ids,
//! duplicate module/command/action ids among the selected, unknown capabilities, incompatible
//! `engine_api`, and no common build target.

use retreat_manifest::capabilities::is_capability;
use retreat_manifest::validate::find_duplicates;
use retreat_manifest::{Manifest, Problem, Target};
use semver::Version;
use std::collections::BTreeSet;
use std::fmt;

/// What to build: the selected modules in selection order plus what they add up to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Selected module ids, in selection order (plugins are added in this order).
    pub modules: Vec<String>,
    /// Plugin type paths, in the same order.
    pub plugins: Vec<String>,
    /// Union of the engine capabilities the modules require, sorted.
    pub capabilities: Vec<String>,
    /// Targets every selected module supports (never empty), in `Native`, `Wasm` order.
    pub targets: Vec<Target>,
    /// True when any selected module requires `networking`.
    pub networked: bool,
}

/// Every reason a selection cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub problems: Vec<Problem>,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = self.problems.iter().map(|p| p.to_string()).collect();
        write!(f, "{}", lines.join("\n"))
    }
}

impl std::error::Error for ResolveError {}

pub fn resolve(
    selection: &[String],
    manifests: &[Manifest],
    engine_version: &Version,
) -> Result<Plan, ResolveError> {
    let mut problems: Vec<Problem> = Vec::new();
    let mut selected: Vec<&Manifest> = Vec::new();

    for (index, id) in selection.iter().enumerate() {
        if selection[..index].contains(id) {
            problems.push(Problem::new(id, "selected twice; list each module once"));
            continue;
        }
        let matches: Vec<&Manifest> = manifests.iter().filter(|m| &m.id == id).collect();
        match matches.as_slice() {
            [] => problems.push(Problem::new(
                id,
                format!(
                    "no module with this id is available; available: {}",
                    available_ids(manifests)
                ),
            )),
            [one] => selected.push(one),
            [first, rest @ ..] => {
                // Two available manifests share the id: the selection is ambiguous.
                selected.push(first);
                selected.extend(rest.iter().copied());
            }
        }
    }

    problems.extend(find_duplicates(&selected));

    for manifest in &selected {
        for name in &manifest.requires {
            if !is_capability(name) {
                problems.push(Problem::new(
                    &manifest.id,
                    format!("requires `{name}`, which is not an engine capability"),
                ));
            }
        }
        if !manifest.engine_api.matches(engine_version) {
            problems.push(Problem::new(
                &manifest.id,
                format!(
                    "needs engine_api `{}` but the engine is {engine_version}; pick a module release that supports this engine, or another engine version",
                    manifest.engine_api
                ),
            ));
        }
    }

    let targets: Vec<Target> = [Target::Native, Target::Wasm]
        .into_iter()
        .filter(|target| selected.iter().all(|m| m.targets.contains(target)))
        .collect();
    if targets.is_empty() && !selected.is_empty() {
        problems.push(Problem::new(
            "<selection>",
            "the selected modules have no build target in common (one is native-only, another wasm-only); remove one of them",
        ));
    }

    if !problems.is_empty() {
        return Err(ResolveError { problems });
    }

    let capabilities: BTreeSet<String> = selected
        .iter()
        .flat_map(|m| m.requires.iter().cloned())
        .collect();
    Ok(Plan {
        modules: selected.iter().map(|m| m.id.clone()).collect(),
        plugins: selected.iter().map(|m| m.plugin.clone()).collect(),
        networked: capabilities.contains("networking"),
        capabilities: capabilities.into_iter().collect(),
        targets,
    })
}

fn available_ids(manifests: &[Manifest]) -> String {
    let mut ids: Vec<&str> = manifests.iter().map(|m| m.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}
