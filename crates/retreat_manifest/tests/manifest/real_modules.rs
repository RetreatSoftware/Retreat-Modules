//! The modules checked into this repository must all satisfy manifest v1.

use retreat_manifest::workspace::validate_workspace;
use std::path::Path;

#[test]
fn every_module_in_this_repository_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = validate_workspace(&root);
    let problems: Vec<String> = report.problems.iter().map(|p| p.to_string()).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(
        report.modules.len() >= 7,
        "expected the seven shipped modules"
    );
}

#[test]
fn a_wrong_root_is_a_plain_error() {
    let report = validate_workspace(Path::new("/definitely/not/here"));
    assert_eq!(report.problems.len(), 1);
    assert!(
        report.problems[0]
            .message
            .contains("Retreat-Modules checkout root")
    );
}
