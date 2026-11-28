//! Resolver tests: manifests are built in code (no files, no Bevy).

use retreat_manifest::{Manifest, NetworkRole, Target};
use retreat_resolver::resolve;
use semver::{Version, VersionReq};

fn manifest(id: &str, commands: &[&str], actions: &[&str], requires: &[&str]) -> Manifest {
    Manifest {
        package_name: format!("retreat_module_{id}"),
        id: id.to_string(),
        display_name: id.to_string(),
        summary: "test".to_string(),
        plugin: format!("retreat_module_{id}::Plugin"),
        commands: commands.iter().map(|s| s.to_string()).collect(),
        actions: actions.iter().map(|s| s.to_string()).collect(),
        requires: requires.iter().map(|s| s.to_string()).collect(),
        engine_api: VersionReq::parse("0.1").unwrap(),
        targets: vec![Target::Native, Target::Wasm],
        network_role: if requires.contains(&"networking") {
            Some(NetworkRole::Both)
        } else {
            None
        },
        conflicts: Vec::new(),
    }
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn engine() -> Version {
    Version::parse("0.1.0").unwrap()
}

fn error_text(selection: &[&str], manifests: &[Manifest], engine_version: &Version) -> String {
    resolve(&ids(selection), manifests, engine_version)
        .expect_err("resolve should fail")
        .to_string()
}

#[test]
fn valid_selection_gives_an_ordered_plan() {
    let all = [
        manifest("a", &["a.go"], &["go"], &["input", "physics"]),
        manifest("b", &["b.go"], &[], &["networking", "input"]),
    ];
    let plan = resolve(&ids(&["b", "a"]), &all, &engine()).unwrap();
    assert_eq!(plan.modules, ids(&["b", "a"]));
    assert_eq!(plan.capabilities, ids(&["input", "networking", "physics"]));
    assert_eq!(plan.targets, vec![Target::Native, Target::Wasm]);
    assert!(plan.networked);
}

#[test]
fn empty_selection_is_an_empty_plan() {
    let plan = resolve(&[], &[], &engine()).unwrap();
    assert!(plan.modules.is_empty());
    assert!(!plan.networked);
}

#[test]
fn unknown_module_lists_the_available_ones() {
    let all = [manifest("a", &[], &[], &[])];
    let text = error_text(&["zzz"], &all, &engine());
    assert!(
        text.contains("`zzz`") && text.contains("available: a"),
        "{text}"
    );
}

#[test]
fn module_selected_twice_is_rejected() {
    let all = [manifest("a", &[], &[], &[])];
    assert!(error_text(&["a", "a"], &all, &engine()).contains("selected twice"));
}

#[test]
fn duplicate_command_between_modules_is_rejected() {
    let all = [
        manifest("a", &["x.go"], &[], &[]),
        manifest("b", &["x.go"], &[], &[]),
    ];
    let text = error_text(&["a", "b"], &all, &engine());
    assert!(text.contains("command `x.go`"), "{text}");
}

#[test]
fn duplicate_action_between_modules_is_rejected() {
    let all = [
        manifest("a", &[], &["jump"], &[]),
        manifest("b", &[], &["jump"], &[]),
    ];
    assert!(error_text(&["a", "b"], &all, &engine()).contains("input action `jump`"));
}

#[test]
fn duplicate_module_id_among_available_manifests_is_rejected() {
    let all = [manifest("a", &[], &[], &[]), manifest("a", &[], &[], &[])];
    assert!(error_text(&["a"], &all, &engine()).contains("module id `a`"));
}

#[test]
fn unknown_capability_is_rejected() {
    let all = [manifest("a", &[], &[], &["teleport"])];
    assert!(error_text(&["a"], &all, &engine()).contains("`teleport`"));
}

#[test]
fn incompatible_engine_api_is_rejected_and_names_both_versions() {
    let all = [manifest("a", &[], &[], &[])];
    let text = error_text(&["a"], &all, &Version::parse("1.0.0").unwrap());
    assert!(text.contains("0.1") && text.contains("1.0.0"), "{text}");
}

#[test]
fn compatible_patch_release_of_the_engine_is_accepted() {
    let all = [manifest("a", &[], &[], &[])];
    assert!(resolve(&ids(&["a"]), &all, &Version::parse("0.1.7").unwrap()).is_ok());
}

#[test]
fn no_common_target_is_rejected() {
    let mut native_only = manifest("a", &[], &[], &[]);
    native_only.targets = vec![Target::Native];
    let mut wasm_only = manifest("b", &[], &[], &[]);
    wasm_only.targets = vec![Target::Wasm];
    let text = error_text(&["a", "b"], &[native_only, wasm_only], &engine());
    assert!(text.contains("no build target in common"), "{text}");
}

#[test]
fn every_problem_is_reported_not_only_the_first() {
    let all = [
        manifest("a", &["x.go"], &[], &["teleport"]),
        manifest("b", &["x.go"], &[], &[]),
    ];
    let error = resolve(&ids(&["a", "b", "missing"]), &all, &engine()).unwrap_err();
    assert!(error.problems.len() >= 3, "{error}");
}

#[test]
fn modules_that_conflict_are_never_selected_together() {
    let mut grid = manifest("grid_inventory", &["bag.put"], &[], &[]);
    grid.conflicts = vec!["inventory".to_string()];
    let stack = manifest("inventory", &["inventory.use"], &[], &[]);
    let other = manifest("menu", &["menu.toggle"], &[], &[]);
    let manifests = [grid, stack, other];
    let text = error_text(
        &["inventory", "menu", "grid_inventory"],
        &manifests,
        &engine(),
    );
    assert!(
        text.contains("cannot be selected together with `grid_inventory`"),
        "{text}"
    );
    assert!(resolve(&ids(&["grid_inventory", "menu"]), &manifests, &engine()).is_ok());
}
