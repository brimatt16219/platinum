//! Snapshot tests for both renderers.
//!
//! `insta::assert_snapshot!` compares output against a stored file in
//! `src/render/snapshots/`. On a mismatch the test fails and insta writes a
//! `.snap.new` file next to it; `cargo insta review` (or reading the diff)
//! shows exactly what changed. That turns "is this output still right?" into
//! a reviewable diff in git.

use serde_json::Value;

use super::{HumanOptions, render_human, render_json};
use crate::codes::Code;
use crate::diagnostic::{Applicability, Diagnostic, Edit, Fix};
use crate::source::{SourceFile, SourceMap};
use crate::span::{FileId, Span};

const BANK_FEE_PATH: &str = "examples/broken/bank_fee.plat";
const BANK_FEE: &str = include_str!("../../../../examples/broken/bank_fee.plat");

/// Span of the first occurrence of `needle` in `text`. Test-only helper; a
/// missing needle means the example file changed and the test must be updated.
fn find(file: FileId, text: &str, needle: &str) -> Span {
    let start = text.find(needle).unwrap_or_else(|| panic!("{needle:?} not found"));
    Span::new(file, start as u32, (start + needle.len()) as u32)
}

/// Recreates the PT3001 example from `spec/05-verification.md` against the
/// real example file. (The verifier that produces it for real arrives in M4.)
fn bank_fee() -> (SourceMap, Diagnostic) {
    let mut sources = SourceMap::new();
    let file = sources.add(SourceFile::new(BANK_FEE_PATH, BANK_FEE).expect("small file"));

    let primary = find(file, BANK_FEE, "acct.balance - amt - FEE");
    let condition = find(file, BANK_FEE, "amt > acct.balance");
    let amt = Span::new(file, condition.start, condition.start + 3);

    let diagnostic = Diagnostic::error(Code(3001), "refinement may not hold", primary)
        .with_primary_label("field `balance` requires `self >= 0` (from type Account)")
        .with_label(condition, "this condition was false")
        .with_note("counterexample: acct.balance = 5, amt = 5, FEE = 1")
        .with_note("so balance would be 5 - 5 - 1 = -1")
        .with_fix(Fix::new(
            "include FEE in the balance check",
            Applicability::MaybeIncorrect,
            vec![Edit::new(amt, "amt + FEE")],
        ));
    (sources, diagnostic)
}

#[test]
fn json_bank_fee() {
    let (sources, d) = bank_fee();
    insta::assert_snapshot!(render_json("plat prove", [&d], &sources));
}

#[test]
fn human_bank_fee() {
    let (sources, d) = bank_fee();
    insta::assert_snapshot!(render_human([&d], &sources, HumanOptions { color: false }));
}

#[test]
fn json_is_valid_and_has_the_v1_envelope() {
    let (sources, d) = bank_fee();
    let json: Value = serde_json::from_str(&render_json("plat prove", [&d], &sources))
        .expect("renderer must produce valid JSON");
    assert_eq!(json["schema"], "platinum.diagnostics/v1");
    assert_eq!(json["tool"], "plat prove");
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(json["diagnostics"][0]["code"], "PT3001");
    assert_eq!(json["diagnostics"][0]["fixes"][0]["id"], "fix-1");
    assert_eq!(json["diagnostics"][0]["docs"], "plat explain PT3001");
}

#[test]
fn json_with_no_diagnostics() {
    let sources = SourceMap::new();
    let json: Value = serde_json::from_str(&render_json("plat check", [], &sources)).expect("valid");
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["summary"]["warnings"], 0);
    assert_eq!(json["diagnostics"], Value::Array(vec![]));
}

#[test]
fn json_columns_count_characters_and_offsets_count_bytes() {
    let mut sources = SourceMap::new();
    let text = "let s = \"€\" $";
    let file = sources.add(SourceFile::new("m.plat", text).expect("small"));
    let dollar = find(file, text, "$");
    let d = Diagnostic::error(Code(1), "unexpected character `$`", dollar);

    let json: Value = serde_json::from_str(&render_json("plat check", [&d], &sources)).expect("valid");
    let start = &json["diagnostics"][0]["primary"]["start"];
    assert_eq!(start["offset"], 14); // `€` is 3 bytes
    assert_eq!(start["col"], 13); // but one character
    assert_eq!(start["line"], 1);
}

#[test]
fn json_survives_a_span_into_an_unknown_file() {
    let sources = SourceMap::new();
    let d = Diagnostic::error(Code(1), "x", Span::new(FileId(9), 0, 1));
    let json: Value = serde_json::from_str(&render_json("plat check", [&d], &sources)).expect("valid");
    assert_eq!(json["diagnostics"][0]["primary"]["file"], "<unknown>");
    assert_eq!(json["diagnostics"][0]["primary"]["start"]["line"], 0);
}

#[test]
fn human_end_of_file_point_and_warning() {
    let mut sources = SourceMap::new();
    let text = "puts(\"hello";
    let file = sources.add(SourceFile::new("eof.plat", text).expect("small"));
    let eof = Span::point(file, text.len() as u32);
    let open = find(file, text, "\"");

    let error = Diagnostic::error(Code(2), "unterminated string literal", open)
        .with_primary_label("this string is never closed")
        .with_label(eof, "file ends here");
    let warning = Diagnostic::warning(Code(1041), "ambiguous `?`", find(file, text, "puts"));

    insta::assert_snapshot!(render_human([&error, &warning], &sources, HumanOptions { color: false }));
}

#[test]
fn human_never_panics_on_unknown_file() {
    let sources = SourceMap::new();
    let d = Diagnostic::error(Code(1), "x", Span::new(FileId(9), 0, 1));
    let out = render_human([&d], &sources, HumanOptions { color: false });
    assert!(out.contains("PT0001"), "got: {out}");
}
