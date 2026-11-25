//! Reads every module of a Retreat-Modules checkout from disk and validates it.
//!
//! Layout contract: `<root>/crates/retreat_module_<id>/{Cargo.toml, src/**/*.rs}`. The contract
//! crates (`dependencies::CONTRACT_CRATES`) are not modules; only their dependencies are checked.

use crate::dependencies::{
    CONTRACT_CRATES, CONTRACT_DEPENDENCIES, MODULE_DEPENDENCIES, check_dependencies,
};
use crate::manifest::Manifest;
use crate::problem::Problem;
use crate::validate::{check_against_source, find_duplicates};
use std::fs;
use std::path::{Path, PathBuf};

/// A module that passed the manifest checks, with where it came from.
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub directory: PathBuf,
    pub manifest: Manifest,
}

/// Result of validating a whole checkout.
#[derive(Debug, Default)]
pub struct WorkspaceReport {
    pub modules: Vec<LoadedModule>,
    pub problems: Vec<Problem>,
}

/// Validates every `crates/retreat_module_*` directory under `root`, plus cross-module
/// uniqueness. Reading problems (missing files) are reported as problems, never panics.
pub fn validate_workspace(root: &Path) -> WorkspaceReport {
    let mut report = WorkspaceReport::default();
    let crates_dir = root.join("crates");
    let entries = match fs::read_dir(&crates_dir) {
        Ok(entries) => entries,
        Err(error) => {
            report.problems.push(Problem::new(
                "<workspace>",
                format!(
                    "cannot read {}: {error}; pass the Retreat-Modules checkout root",
                    crates_dir.display()
                ),
            ));
            return report;
        }
    };
    let mut directories: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("retreat_module_"))
        })
        .collect();
    directories.sort();
    for directory in directories {
        load_module(&directory, &mut report);
    }
    for name in CONTRACT_CRATES {
        check_contract_crate(&crates_dir.join(name), name, &mut report);
    }
    let manifests: Vec<&Manifest> = report.modules.iter().map(|m| &m.manifest).collect();
    let duplicates = find_duplicates(&manifests);
    report.problems.extend(duplicates);
    report
}

fn load_module(directory: &Path, report: &mut WorkspaceReport) {
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<module>")
        .to_string();
    let cargo_toml = match fs::read_to_string(directory.join("Cargo.toml")) {
        Ok(text) => text,
        Err(error) => {
            report.problems.push(Problem::new(
                &name,
                format!("cannot read Cargo.toml: {error}"),
            ));
            return;
        }
    };
    report
        .problems
        .extend(check_dependencies(&name, &cargo_toml, MODULE_DEPENDENCIES));
    let manifest = match Manifest::from_cargo_toml(&cargo_toml) {
        Ok(manifest) => manifest,
        Err(problems) => {
            report.problems.extend(problems);
            return;
        }
    };
    if manifest.package_name != name {
        report.problems.push(Problem::new(
            &manifest.id,
            format!(
                "directory `{name}` does not match the package name `{}`",
                manifest.package_name
            ),
        ));
    }
    match read_sources(&directory.join("src")) {
        Ok(source) => report
            .problems
            .extend(check_against_source(&manifest, &source)),
        Err(error) => report.problems.push(Problem::new(
            &manifest.id,
            format!("cannot read the module source in src/: {error}"),
        )),
    }
    report.modules.push(LoadedModule {
        directory: directory.to_path_buf(),
        manifest,
    });
}

/// A contract crate may be absent (a test checkout), but when present it must stay below modules.
fn check_contract_crate(directory: &Path, name: &str, report: &mut WorkspaceReport) {
    if !directory.is_dir() {
        return;
    }
    match fs::read_to_string(directory.join("Cargo.toml")) {
        Ok(text) => report
            .problems
            .extend(check_dependencies(name, &text, CONTRACT_DEPENDENCIES)),
        Err(error) => report.problems.push(Problem::new(
            name,
            format!("cannot read Cargo.toml: {error}"),
        )),
    }
}

/// Concatenates every `.rs` file under `dir` (sorted, so the result is deterministic).
fn read_sources(dir: &Path) -> std::io::Result<String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_rs_files(dir, &mut paths)?;
    paths.sort();
    let mut text = String::new();
    for path in paths {
        text.push_str(&fs::read_to_string(path)?);
        text.push('\n');
    }
    Ok(text)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}
