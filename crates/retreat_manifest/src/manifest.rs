//! Manifest v1: the parsed and shape-checked `[package.metadata.retreat]` table.
//!
//! Schema (all keys required unless marked optional):
//!
//! | key | meaning |
//! |---|---|
//! | `manifest_version` | must be `1` |
//! | `id` | module id, `[a-z][a-z0-9_]*`; the crate is `retreat_module_<id>` |
//! | `display_name`, `summary` | non-empty text shown in Forge |
//! | `plugin` | `<crate>::<Type>`: the Bevy plugin a game adds |
//! | `commands` | command ids it registers, each `area.verb` (may be empty) |
//! | `actions` | optional; input action ids it registers (`[a-z][a-z0-9_]*`) |
//! | `requires` | engine capabilities (see `capabilities`) |
//! | `engine_api` | semver requirement on `retreat_engine_api`, e.g. `"0.1"` |
//! | `targets` | `"native"` and/or `"wasm"` |
//! | `network_role` | `"server"`, `"client"` or `"both"`; required if and only if `requires` has `networking` |
//! | `config` | optional placeholder table; must be empty in v1 (reserved for configuration) |

use crate::capabilities::is_capability;
use crate::problem::Problem;
use semver::VersionReq;
use serde::Deserialize;

/// The only manifest version this crate reads.
pub const MANIFEST_VERSION: u32 = 1;

/// Where a networked module's code runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkRole {
    Server,
    Client,
    Both,
}

/// A platform a module builds for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Native,
    Wasm,
}

/// A manifest that passed the shape checks (see `Manifest::from_cargo_toml`).
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub package_name: String,
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub plugin: String,
    pub commands: Vec<String>,
    pub actions: Vec<String>,
    pub requires: Vec<String>,
    pub engine_api: VersionReq,
    pub targets: Vec<Target>,
    pub network_role: Option<NetworkRole>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    manifest_version: u32,
    id: String,
    display_name: String,
    summary: String,
    plugin: String,
    commands: Vec<String>,
    #[serde(default)]
    actions: Vec<String>,
    requires: Vec<String>,
    engine_api: String,
    targets: Vec<Target>,
    network_role: Option<NetworkRole>,
    #[serde(default)]
    config: toml::Table,
}

/// `[a-z][a-z0-9_]*`
pub fn is_snake_id(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

/// `area.verb`, both parts `[a-z][a-z0-9_]*`.
pub fn is_command_id(text: &str) -> bool {
    match text.split_once('.') {
        Some((area, verb)) => is_snake_id(area) && is_snake_id(verb),
        None => false,
    }
}

impl Manifest {
    /// Parses the `[package.metadata.retreat]` table of a module's `Cargo.toml` text and checks its
    /// shape (formats, known capabilities, role rules). It does not look at the module's source:
    /// that is `validate::check_against_source`. Returns every problem found, not only the first.
    pub fn from_cargo_toml(cargo_toml: &str) -> Result<Manifest, Vec<Problem>> {
        let document: toml::Table = cargo_toml.parse().map_err(|error| {
            vec![Problem::new(
                "<unknown>",
                format!("Cargo.toml is not valid TOML: {error}"),
            )]
        })?;
        let package_name = document
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(|name| name.as_str())
            .ok_or_else(|| {
                vec![Problem::new(
                    "<unknown>",
                    "Cargo.toml has no [package] name; a module is a package",
                )]
            })?
            .to_string();
        let table = document
            .get("package")
            .and_then(|package| package.get("metadata"))
            .and_then(|metadata| metadata.get("retreat"))
            .ok_or_else(|| {
                vec![Problem::new(
                    &package_name,
                    "no [package.metadata.retreat] table; add the module manifest \
                     (schema: Retreat-Modules/CLAUDE.md, manifest v1)",
                )]
            })?
            .clone();
        let raw: RawManifest = table.try_into().map_err(|error| {
            vec![Problem::new(
                &package_name,
                format!("invalid [package.metadata.retreat]: {error}"),
            )]
        })?;
        Self::from_raw(package_name, raw)
    }

    fn from_raw(package_name: String, raw: RawManifest) -> Result<Manifest, Vec<Problem>> {
        let who = if raw.id.is_empty() {
            package_name.clone()
        } else {
            raw.id.clone()
        };
        let mut problems: Vec<String> = Vec::new();

        if raw.manifest_version != MANIFEST_VERSION {
            problems.push(format!(
                "manifest_version is {}, this validator reads {MANIFEST_VERSION}; set manifest_version = {MANIFEST_VERSION}",
                raw.manifest_version
            ));
        }
        if !is_snake_id(&raw.id) {
            problems.push(format!(
                "id `{}` must be lowercase letters, digits and `_`, starting with a letter",
                raw.id
            ));
        } else if package_name != format!("retreat_module_{}", raw.id) {
            problems.push(format!(
                "id `{}` does not match the crate name `{package_name}`; the crate must be named `retreat_module_{}`",
                raw.id, raw.id
            ));
        }
        for (key, value) in [
            ("display_name", &raw.display_name),
            ("summary", &raw.summary),
        ] {
            if value.trim().is_empty() {
                problems.push(format!("{key} must not be empty"));
            }
        }
        match raw.plugin.split_once("::") {
            Some((crate_name, type_name))
                if crate_name == package_name
                    && !type_name.is_empty()
                    && !type_name.contains("::") => {}
            _ => problems.push(format!(
                "plugin `{}` must be `{package_name}::<PluginType>`",
                raw.plugin
            )),
        }
        problems.extend(check_list(
            "commands",
            &raw.commands,
            is_command_id,
            "`area.verb` in lowercase snake_case",
        ));
        problems.extend(check_list(
            "actions",
            &raw.actions,
            is_snake_id,
            "lowercase snake_case",
        ));
        problems.extend(check_list("requires", &raw.requires, |_| true, "a name"));
        for name in &raw.requires {
            if !is_capability(name) {
                problems.push(format!(
                    "requires `{name}` is not an engine capability; known: {}",
                    crate::capabilities::CAPABILITIES.join(", ")
                ));
            }
        }
        let engine_api = match VersionReq::parse(&raw.engine_api) {
            Ok(req) => Some(req),
            Err(error) => {
                problems.push(format!(
                    "engine_api `{}` is not a semver requirement ({error}); write e.g. \"0.1\"",
                    raw.engine_api
                ));
                None
            }
        };
        if raw.targets.is_empty() {
            problems.push("targets must list at least one of \"native\", \"wasm\"".to_string());
        }
        for (index, target) in raw.targets.iter().enumerate() {
            if raw.targets[..index].contains(target) {
                problems.push(format!("targets lists {target:?} twice"));
            }
        }
        let needs_role = raw.requires.iter().any(|name| name == "networking");
        match (needs_role, raw.network_role) {
            (true, None) => problems.push(
                "requires `networking`, so network_role (\"server\", \"client\" or \"both\") is required"
                    .to_string(),
            ),
            (false, Some(_)) => problems.push(
                "network_role is set but requires has no `networking`; remove network_role or add the capability"
                    .to_string(),
            ),
            _ => {}
        }
        if !raw.config.is_empty() {
            problems.push(
                "config is a reserved placeholder and must be empty in manifest v1".to_string(),
            );
        }

        match (problems.is_empty(), engine_api) {
            (true, Some(engine_api)) => Ok(Manifest {
                package_name,
                id: raw.id,
                display_name: raw.display_name,
                summary: raw.summary,
                plugin: raw.plugin,
                commands: raw.commands,
                actions: raw.actions,
                requires: raw.requires,
                engine_api,
                targets: raw.targets,
                network_role: raw.network_role,
            }),
            _ => Err(problems
                .into_iter()
                .map(|message| Problem::new(&who, message))
                .collect()),
        }
    }
}

/// Checks that each entry has the right format and that there are no duplicates.
fn check_list(
    key: &str,
    values: &[String],
    is_valid: impl Fn(&str) -> bool,
    expected: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, value) in values.iter().enumerate() {
        if !is_valid(value) {
            problems.push(format!("{key} entry `{value}` must be {expected}"));
        }
        if values[..index].contains(value) {
            problems.push(format!("{key} lists `{value}` twice"));
        }
    }
    problems
}
