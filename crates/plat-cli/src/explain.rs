//! `plat explain PT0002`: print the long explanation for a diagnostic code.
//!
//! The text comes from the registry in `plat-diag/src/codes.rs`, the same
//! place every diagnostic's code is defined, so an explanation exists for
//! every code the compiler can produce (a test in `codes.rs` enforces it).

use std::process::ExitCode;

use plat_diag::Code;
use serde_json::json;

use crate::EXIT_USAGE;

/// Schema identifier for `plat explain --json` (documented in spec/09-tooling.md).
const SCHEMA: &str = "platinum.explain/v1";

pub fn run(input: &str, json: bool) -> ExitCode {
    let code: Code = match input.parse() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let Some(info) = code.info() else {
        eprintln!("error: {code} is not a known diagnostic code");
        return ExitCode::from(EXIT_USAGE);
    };

    if json {
        let doc = json!({
            "schema": SCHEMA,
            "code": code.to_string(),
            "title": info.title,
            "area": code.area(),
            "explanation": info.explanation,
        });
        println!("{doc:#}");
    } else {
        let area = code.area().map(|a| format!(" ({a})")).unwrap_or_default();
        println!("{code}: {}{area}\n\n{}", info.title, info.explanation);
    }
    ExitCode::SUCCESS
}
