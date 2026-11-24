use retreat_manifest::{Manifest, NetworkRole, Target};

const VALID: &str = r#"
[package]
name = "retreat_module_demo"

[package.metadata.retreat]
manifest_version = 1
id = "demo"
display_name = "Demo"
summary = "A demo."
plugin = "retreat_module_demo::DemoModulePlugin"
commands = ["demo.go"]
actions = ["go"]
requires = ["input", "networking"]
engine_api = "0.1"
targets = ["native", "wasm"]
network_role = "both"
"#;

fn errors(cargo_toml: &str) -> String {
    Manifest::from_cargo_toml(cargo_toml)
        .expect_err("manifest should be invalid")
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn with(from: &str, to: &str) -> String {
    assert!(VALID.contains(from), "test bug: {from} not in VALID");
    VALID.replace(from, to)
}

#[test]
fn valid_manifest_parses() {
    let manifest = Manifest::from_cargo_toml(VALID).unwrap();
    assert_eq!(manifest.id, "demo");
    assert_eq!(manifest.targets, [Target::Native, Target::Wasm]);
    assert_eq!(manifest.network_role, Some(NetworkRole::Both));
}

#[test]
fn missing_metadata_table_says_where_to_add_it() {
    let text = errors("[package]\nname = \"retreat_module_demo\"\n");
    assert!(text.contains("[package.metadata.retreat]"), "{text}");
}

#[test]
fn unknown_key_is_rejected() {
    let text = errors(&with("id = \"demo\"", "id = \"demo\"\nsurprise = 1"));
    assert!(text.contains("surprise"), "{text}");
}

#[test]
fn wrong_manifest_version_is_rejected() {
    assert!(
        errors(&with("manifest_version = 1", "manifest_version = 2")).contains("manifest_version")
    );
}

#[test]
fn unknown_capability_lists_the_known_ones() {
    let text = errors(&with(
        "\"input\", \"networking\"",
        "\"input\", \"networking\", \"teleport\"",
    ));
    assert!(
        text.contains("`teleport`") && text.contains("physics"),
        "{text}"
    );
}

#[test]
fn id_must_match_the_crate_name() {
    assert!(errors(&with("id = \"demo\"", "id = \"other\"")).contains("retreat_module_other"));
}

#[test]
fn plugin_must_live_in_this_crate() {
    assert!(
        errors(&with(
            "retreat_module_demo::DemoModulePlugin",
            "elsewhere::P"
        ))
        .contains("plugin")
    );
}

#[test]
fn command_must_be_area_dot_verb() {
    assert!(errors(&with("\"demo.go\"", "\"demogo\"")).contains("area.verb"));
}

#[test]
fn duplicates_inside_a_list_are_rejected() {
    assert!(errors(&with("actions = [\"go\"]", "actions = [\"go\", \"go\"]")).contains("twice"));
}

#[test]
fn engine_api_must_be_semver() {
    assert!(errors(&with("engine_api = \"0.1\"", "engine_api = \"banana\"")).contains("semver"));
}

#[test]
fn networking_requires_a_network_role() {
    assert!(errors(&with("network_role = \"both\"", "")).contains("network_role"));
}

#[test]
fn network_role_without_networking_is_rejected() {
    assert!(errors(&with("\"input\", \"networking\"", "\"input\"")).contains("network_role"));
}

#[test]
fn targets_must_not_be_empty() {
    assert!(errors(&with("targets = [\"native\", \"wasm\"]", "targets = []")).contains("targets"));
}

#[test]
fn config_placeholder_must_be_empty() {
    let text = errors(&format!(
        "{VALID}\n[package.metadata.retreat.config]\nx = 1\n"
    ));
    assert!(text.contains("config"), "{text}");
}

#[test]
fn empty_config_placeholder_is_accepted() {
    let text = format!("{VALID}\n[package.metadata.retreat.config]\n");
    assert!(Manifest::from_cargo_toml(&text).is_ok());
}
