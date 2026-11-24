use retreat_manifest::Manifest;
use retreat_manifest::validate::{check_against_source, find_duplicates};

fn demo(id: &str, commands: &str, actions: &str) -> Manifest {
    let text = format!(
        r#"
[package]
name = "retreat_module_{id}"
[package.metadata.retreat]
manifest_version = 1
id = "{id}"
display_name = "D"
summary = "S"
plugin = "retreat_module_{id}::P"
commands = [{commands}]
actions = [{actions}]
requires = ["input"]
engine_api = "0.1"
targets = ["native"]
"#
    );
    Manifest::from_cargo_toml(&text).unwrap()
}

const SOURCE: &str = r#"
pub struct P;
impl Plugin for P {
    fn build(&self, app: &mut App) {
        app.register_command("d.go", "d.go", f)
           .register_action(InputAction { id: "go", command_line: "d.go", default_keys: &[] });
    }
}
"#;

#[test]
fn matching_source_has_no_problems() {
    assert!(check_against_source(&demo("d", "\"d.go\"", "\"go\""), SOURCE).is_empty());
}

#[test]
fn command_in_manifest_but_not_in_source() {
    let problems = check_against_source(&demo("d", "\"d.go\", \"d.ghost\"", "\"go\""), SOURCE);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("d.ghost"));
}

#[test]
fn command_in_source_but_not_in_manifest() {
    let problems = check_against_source(&demo("d", "", "\"go\""), SOURCE);
    assert!(problems[0].message.contains("`d.go`") && problems[0].message.contains("add it"));
}

#[test]
fn action_mismatch_is_reported() {
    let problems = check_against_source(&demo("d", "\"d.go\"", "\"other\""), SOURCE);
    assert_eq!(problems.len(), 2);
}

#[test]
fn missing_plugin_type_is_reported() {
    let problems = check_against_source(&demo("d", "", ""), "");
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("plugin type `P`"))
    );
}

#[test]
fn duplicates_across_modules_are_found() {
    let a = demo("a", "\"x.go\"", "\"go\"");
    let b = demo("b", "\"x.go\"", "\"go\"");
    let problems = find_duplicates(&[&a, &b]);
    assert_eq!(problems.len(), 2);
    assert!(
        find_duplicates(&[&a, &a])
            .iter()
            .any(|p| p.message.contains("module id"))
    );
}
