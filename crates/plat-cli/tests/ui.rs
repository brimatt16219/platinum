//! # Golden ("UI") tests
//!
//! Every `tests/ui/**/foo.plat` file is a test case. The harness runs
//! `plat check --json` on it and compares the output with the expected file
//! next to it, `foo.diag.json`. Any difference fails the test with a readable
//! report. This is the format CLAUDE.md asks every feature to land with, and
//! it tests the compiler exactly as an agent sees it: through the JSON.
//!
//! The idea comes from rustc's UI tests: inputs and expected outputs are
//! ordinary files, so adding a test means adding a file, and reviewing a
//! behavior change means reading a diff of expected outputs.
//!
//! ## Blessing
//!
//! When output changes on purpose, regenerate the expected files with:
//!
//! ```text
//! PLAT_BLESS=1 cargo test -p plat-cli --test ui        (bash)
//! $env:PLAT_BLESS=1; cargo test -p plat-cli --test ui  (PowerShell)
//! ```
//!
//! then review the diff in git before committing. Never bless blindly: the
//! expected files are the specification of what users see.
//!
//! ## Coverage
//!
//! `every_lexer_code_has_a_golden_test` checks that each code in the lexing
//! range of the registry is produced by at least one golden file, so a new
//! diagnostic cannot land without a test. Later milestones extend it to
//! their own ranges.
//!
//! Later milestones will add `foo.stdout` (expected program output from
//! `plat run`); the harness will pick whichever expected files exist.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use plat_diag::codes::REGISTRY;
use serde_json::Value;

/// `tests/ui` at the workspace root.
fn ui_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ui")
}

/// Every `.plat` file under `tests/ui`, as a path relative to it using `/`
/// separators (so file names inside the JSON are the same on every OS).
fn cases() -> Vec<String> {
    let root = ui_root();
    let mut found = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("readable directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "plat") {
                let rel = path.strip_prefix(&root).expect("under the ui root");
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                found.push(parts.join("/"));
            }
        }
    }
    found.sort();
    found
}

/// Runs `plat check --json <case>` from the ui root; returns (exit code, JSON).
fn check(case: &str) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_plat"))
        .args(["check", "--json", case])
        .current_dir(ui_root())
        .output()
        .expect("failed to run plat");
    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("{case}: stdout is not JSON ({e})\n{stdout}"));
    (out.status.code().expect("normal exit"), json)
}

fn expected_path(case: &str) -> PathBuf {
    ui_root().join(case.replace(".plat", ".diag.json"))
}

fn blessing() -> bool {
    std::env::var_os("PLAT_BLESS").is_some_and(|v| !v.is_empty() && v != "0")
}

#[test]
fn golden_files_match() {
    let cases = cases();
    assert!(
        !cases.is_empty(),
        "no golden tests found under {}",
        ui_root().display()
    );

    let mut failures = Vec::new();
    for case in &cases {
        let (code, actual) = check(case);

        // The exit code must agree with the diagnostics.
        let has_errors = actual["summary"]["errors"].as_u64().unwrap_or(0) > 0;
        let expected_code = if has_errors { 1 } else { 0 };
        if code != expected_code {
            failures.push(format!(
                "{case}: exit code {code}, expected {expected_code}"
            ));
        }

        let path = expected_path(case);
        let pretty = format!(
            "{}\n",
            serde_json::to_string_pretty(&actual).expect("serializable")
        );

        if blessing() {
            std::fs::write(&path, &pretty)
                .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
            continue;
        }

        let Ok(expected_text) = std::fs::read_to_string(&path) else {
            failures.push(format!(
                "{case}: no expected file {}\n  run with PLAT_BLESS=1 to create it, then review it",
                path.display()
            ));
            continue;
        };
        // Compare as JSON values, so whitespace and line endings never matter.
        let expected: Value = match serde_json::from_str(&expected_text) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{case}: expected file is not valid JSON: {e}"));
                continue;
            }
        };
        if actual != expected {
            failures.push(format!(
                "{case}: output differs from {}\n--- expected\n{}\n--- actual\n{pretty}",
                path.display(),
                serde_json::to_string_pretty(&expected).expect("serializable"),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} golden tests failed:\n\n{}\n\nIf the change is intended, rerun with PLAT_BLESS=1 and review the diff.",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}

/// Every lexer diagnostic code (PT0001-PT0099) in the registry must be
/// produced by at least one golden file.
#[test]
fn every_lexer_code_has_a_golden_test() {
    let mut produced = BTreeSet::new();
    for case in cases() {
        let (_, json) = check(&case);
        for d in json["diagnostics"].as_array().into_iter().flatten() {
            if let Some(code) = d["code"].as_str() {
                produced.insert(code.to_string());
            }
        }
    }
    let missing: Vec<String> = REGISTRY
        .iter()
        .filter(|info| info.code.area() == Some("lexing"))
        .map(|info| info.code.to_string())
        .filter(|code| !produced.contains(code))
        .collect();
    assert!(
        missing.is_empty(),
        "lexer codes with no golden test in tests/ui: {missing:?}"
    );
}

/// Expected files without a matching `.plat` are leftovers from a renamed or
/// deleted test.
#[test]
fn no_orphaned_expected_files() {
    let cases: BTreeSet<String> = cases().into_iter().collect();
    let mut orphans = Vec::new();
    let mut pending = vec![ui_root()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("readable dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.to_string_lossy().ends_with(".diag.json") {
                let plat = path.to_string_lossy().replace(".diag.json", ".plat");
                let rel = Path::new(&plat)
                    .strip_prefix(ui_root())
                    .expect("under root");
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                if !cases.contains(&rel) {
                    orphans.push(path.display().to_string());
                }
            }
        }
    }
    assert!(
        orphans.is_empty(),
        "expected files with no .plat test: {orphans:?}"
    );
}
