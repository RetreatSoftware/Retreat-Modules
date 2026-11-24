//! `cargo run -p retreat_manifest --bin check_manifests [root]`
//!
//! Validates every module manifest under `root` (default: the current directory) against the
//! module source. Prints one line per module and every problem; exit code 1 when any problem.

use retreat_manifest::workspace::validate_workspace;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let report = validate_workspace(&root);
    for module in &report.modules {
        println!(
            "checked `{}` ({})",
            module.manifest.id,
            module.directory.display()
        );
    }
    for problem in &report.problems {
        eprintln!("ERROR {problem}");
    }
    if report.problems.is_empty() {
        println!("{} module manifests are valid", report.modules.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("{} problem(s) found", report.problems.len());
        ExitCode::FAILURE
    }
}
