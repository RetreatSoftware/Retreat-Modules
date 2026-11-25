//! The dependency rule: modules never depend on each other.

use retreat_manifest::dependencies::{
    CONTRACT_DEPENDENCIES, MODULE_DEPENDENCIES, check_dependencies,
};

#[test]
fn bevy_and_contract_crates_are_allowed() {
    let text = r#"
[dependencies]
bevy.workspace = true
retreat_engine_api.workspace = true
retreat_gameplay_api.workspace = true
"#;
    assert!(check_dependencies("a", text, MODULE_DEPENDENCIES).is_empty());
}

#[test]
fn another_module_is_refused_in_every_table_even_renamed() {
    for text in [
        "[dependencies]\nretreat_module_health.workspace = true",
        "[dev-dependencies]\nretreat_module_health = { path = \"../h\" }",
        "[build-dependencies]\nretreat_module_health = \"1\"",
        "[target.'cfg(unix)'.dependencies]\nretreat_module_health = \"1\"",
        "[dependencies]\nbevy = { package = \"retreat_module_health\", path = \"../h\" }",
    ] {
        let problems = check_dependencies("a", text, MODULE_DEPENDENCIES);
        assert_eq!(problems.len(), 1, "{text}");
        assert!(
            problems[0].message.contains("`retreat_module_health`"),
            "{}",
            problems[0]
        );
    }
}

#[test]
fn a_contract_crate_may_not_depend_on_a_module_or_the_engine() {
    let text = "[dependencies]\nretreat_module_health = \"1\"\nretreat_engine = \"1\"";
    assert_eq!(
        check_dependencies("c", text, CONTRACT_DEPENDENCIES).len(),
        2
    );
}
