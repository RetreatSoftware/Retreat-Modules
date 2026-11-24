//! Checks of one manifest against its module's source, and across modules.

use crate::manifest::Manifest;
use crate::problem::Problem;
use crate::source_scan::{declares_plugin, registered_actions, registered_commands};
use std::collections::BTreeSet;

/// The manifest must list exactly the commands and actions the source registers, and the plugin
/// type must exist and implement `Plugin`. `source` is the concatenated text of the module's
/// `src/**/*.rs`.
pub fn check_against_source(manifest: &Manifest, source: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let id = &manifest.id;
    let commands = registered_commands(source);
    let actions = registered_actions(source);
    diff(
        id,
        "command",
        "commands",
        "register_command",
        &manifest.commands,
        &commands,
        &mut problems,
    );
    diff(
        id,
        "action",
        "actions",
        "register_action",
        &manifest.actions,
        &actions,
        &mut problems,
    );
    let type_name = manifest.plugin.rsplit("::").next().unwrap_or("");
    if !declares_plugin(source, type_name) {
        problems.push(Problem::new(
            id,
            format!(
                "plugin type `{type_name}` not found: the source needs `pub struct {type_name}` and `impl Plugin for {type_name}`; fix `plugin` in the manifest or the code"
            ),
        ));
    }
    problems
}

fn diff(
    id: &str,
    singular: &str,
    key: &str,
    call: &str,
    declared: &[String],
    found: &[String],
    problems: &mut Vec<Problem>,
) {
    for name in declared {
        if !found.contains(name) {
            problems.push(Problem::new(
                id,
                format!(
                    "manifest lists {singular} `{name}` but the source never calls {call}(\"{name}\"...); remove it from `{key}` or register it"
                ),
            ));
        }
    }
    for name in found {
        if !declared.contains(name) {
            problems.push(Problem::new(
                id,
                format!(
                    "the source registers {singular} `{name}` but the manifest `{key}` does not list it; add it"
                ),
            ));
        }
    }
}

/// Module ids, command ids and action ids must each be unique across `manifests`.
/// Used by the workspace validator (all modules) and by the resolver (the selection).
pub fn find_duplicates(manifests: &[&Manifest]) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut ids = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut actions = BTreeSet::new();
    for manifest in manifests {
        if !ids.insert(manifest.id.clone()) {
            problems.push(Problem::new(
                &manifest.id,
                format!(
                    "module id `{}` is used by more than one module; ids must be unique",
                    manifest.id
                ),
            ));
        }
        for command in &manifest.commands {
            if !commands.insert(command.clone()) {
                problems.push(Problem::new(
                    &manifest.id,
                    format!("command `{command}` is also registered by another module; command ids must be unique"),
                ));
            }
        }
        for action in &manifest.actions {
            if !actions.insert(action.clone()) {
                problems.push(Problem::new(
                    &manifest.id,
                    format!("input action `{action}` is also registered by another module; action ids must be unique"),
                ));
            }
        }
    }
    problems
}
