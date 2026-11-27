//! End-to-end tests for the `plat` binary.
//!
//! These run the real executable as a child process (Cargo builds it and
//! exposes its path as `CARGO_BIN_EXE_plat`), exactly as a user or an agent
//! would, and check stdout, stderr, and the exit code. They test the CLI
//! contract from `spec/09-tooling.md`: JSON on stdout, human output on stderr,
//! exit codes 0 / 1 / 2.
//!
//! Commands run from `tests/fixtures/` with relative file names, so paths in
//! the output (and in snapshots) are short and the same on every machine.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Runs `plat <args>` inside `tests/fixtures/`.
fn plat(args: &[&str]) -> Output {
    plat_in(&fixtures(), args)
}

fn plat_in(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_plat"))
        .args(args)
        .current_dir(dir)
        .env_remove("NO_COLOR")
        .output()
        .expect("failed to run the plat binary");
    Output {
        code: out.status.code().expect("plat exited normally"),
        stdout: String::from_utf8(out.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(out.stderr).expect("stderr is UTF-8"),
    }
}

fn json(stdout: &str) -> Value {
    serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("stdout is not one JSON document ({e}):\n{stdout}"))
}

// --- plat check ----------------------------------------------------------

#[test]
fn check_clean_file() {
    let out = plat(&["check", "clean.plat"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "human mode keeps stdout empty");
    assert_eq!(out.stderr.trim(), "plat check: no problems found in 1 file");
}

#[test]
fn check_clean_file_json() {
    let out = plat(&["check", "--json", "clean.plat"]);
    assert_eq!(out.code, 0);
    assert!(
        out.stderr.is_empty(),
        "JSON mode writes nothing to stderr: {}",
        out.stderr
    );
    let doc = json(&out.stdout);
    assert_eq!(doc["schema"], "platinum.diagnostics/v1");
    assert_eq!(doc["tool"], "plat check");
    assert_eq!(doc["summary"]["errors"], 0);
    assert_eq!(doc["diagnostics"], Value::Array(vec![]));
}

#[test]
fn check_errors_json() {
    let out = plat(&["check", "--json", "lex_errors.plat"]);
    assert_eq!(out.code, 1, "errors mean exit code 1");
    let doc = json(&out.stdout);
    let codes: Vec<&str> = doc["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|d| d["code"].as_str().expect("code string"))
        .collect();
    assert_eq!(codes, ["PT0001", "PT0007", "PT0005"], "sorted by position");
    assert_eq!(doc["summary"]["errors"], 3);
    assert_eq!(doc["diagnostics"][0]["primary"]["file"], "lex_errors.plat");
    assert_eq!(doc["diagnostics"][0]["primary"]["start"]["line"], 1);
    assert_eq!(doc["diagnostics"][0]["primary"]["start"]["col"], 13);
}

#[test]
fn check_errors_human() {
    let out = plat(&["check", "--color", "never", "lex_errors.plat"]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.is_empty());
    insta::assert_snapshot!(out.stderr);
}

#[test]
fn check_several_files_reports_all() {
    let out = plat(&["check", "--json", "clean.plat", "lex_errors.plat"]);
    assert_eq!(out.code, 1);
    let doc = json(&out.stdout);
    assert_eq!(doc["summary"]["errors"], 3);
    assert!(
        doc["diagnostics"]
            .as_array()
            .expect("array")
            .iter()
            .all(|d| d["primary"]["file"] == "lex_errors.plat")
    );
}

#[test]
fn check_all_examples_from_the_spec() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let out = plat_in(
        &examples,
        &[
            "check",
            "hello.plat",
            "shapes.plat",
            "bank.plat",
            "refund.plat",
            "dashboard.plat",
            "broken/bank_fee.plat",
            "broken/missing_needs.plat",
            "broken/non_exhaustive.plat",
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stderr.contains("no problems found in 8 files"),
        "{}",
        out.stderr
    );
}

#[test]
fn check_missing_file_is_a_usage_error() {
    let out = plat(&["check", "does_not_exist.plat"]);
    assert_eq!(out.code, 2);
    assert!(
        out.stderr.contains("cannot read `does_not_exist.plat`"),
        "{}",
        out.stderr
    );
}

#[test]
fn check_needs_at_least_one_file() {
    let out = plat(&["check"]);
    assert_eq!(
        out.code, 2,
        "clap reports missing arguments with exit code 2"
    );
}

// --- plat explain ----------------------------------------------------------

#[test]
fn explain_human() {
    let out = plat(&["explain", "PT0002"]);
    assert_eq!(out.code, 0);
    assert!(
        out.stdout
            .starts_with("PT0002: unterminated string literal (lexing)\n\n"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("Example:"));
}

#[test]
fn explain_json() {
    let out = plat(&["explain", "--json", "pt0005"]);
    assert_eq!(out.code, 0);
    let doc = json(&out.stdout);
    assert_eq!(doc["schema"], "platinum.explain/v1");
    assert_eq!(doc["code"], "PT0005", "normalized to canonical form");
    assert_eq!(doc["title"], "malformed number literal");
    assert_eq!(doc["area"], "lexing");
    assert!(
        doc["explanation"]
            .as_str()
            .expect("string")
            .contains("0b102")
    );
}

#[test]
fn explain_rejects_malformed_and_unknown_codes() {
    let bad = plat(&["explain", "PT1"]);
    assert_eq!(bad.code, 2);
    assert!(
        bad.stderr.contains("expected the form PT0001"),
        "{}",
        bad.stderr
    );

    let unknown = plat(&["explain", "PT0999"]);
    assert_eq!(unknown.code, 2);
    assert!(
        unknown.stderr.contains("not a known diagnostic code"),
        "{}",
        unknown.stderr
    );
}

// --- not yet implemented -----------------------------------------------------

#[test]
fn run_and_fmt_say_when_they_arrive() {
    for args in [["run", "clean.plat"], ["fmt", "clean.plat"]] {
        let out = plat(&args);
        assert_eq!(out.code, 2);
        assert!(out.stderr.contains("arrives in M1"), "{}", out.stderr);
    }
}
