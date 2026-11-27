//! `plat check`: find errors without running anything.
//!
//! The pipeline grows milestone by milestone. In M0 it is:
//!
//! ```text
//! read each file -> SourceMap -> lex -> collect diagnostics -> sort -> render
//! ```
//!
//! M1 adds parsing after lexing, M2 type checking, and so on. Whatever the
//! phases, the shape stays the same: every phase pushes into one
//! `DiagnosticBag`, and rendering happens once at the end.

use std::path::PathBuf;
use std::process::ExitCode;

use plat_diag::{DiagnosticBag, HumanOptions, SourceFile, SourceMap, render_human, render_json};
use plat_syntax::lex;

use crate::EXIT_USAGE;

pub fn run(files: &[PathBuf], json: bool, color: bool) -> ExitCode {
    let mut sources = SourceMap::new();
    let mut diagnostics = DiagnosticBag::new();

    for path in files {
        // Problems reading input are usage errors (exit 2), not diagnostics:
        // there is no source text for a diagnostic to point into.
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("error: cannot read `{}`: {e}", path.display());
                return ExitCode::from(EXIT_USAGE);
            }
        };
        let file = match SourceFile::new(path.display().to_string(), text) {
            Ok(file) => file,
            Err(e) => {
                eprintln!(
                    "error: `{}` is too large ({} bytes; the limit is 4 GiB)",
                    e.name, e.len
                );
                return ExitCode::from(EXIT_USAGE);
            }
        };

        let id = sources.add(file);
        if let Some(file) = sources.get(id) {
            let mut lexed = lex(id, file);
            diagnostics.append(&mut lexed.diagnostics);
        }
    }

    diagnostics.sort();

    if json {
        println!("{}", render_json("plat check", &diagnostics, &sources));
    } else {
        if !diagnostics.is_empty() {
            eprint!(
                "{}",
                render_human(&diagnostics, &sources, HumanOptions { color })
            );
        }
        eprintln!("{}", summary(&diagnostics, files.len()));
    }

    if diagnostics.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// A one-line summary for human output, such as
/// "plat check: 2 errors, 1 warning in 1 file".
fn summary(diagnostics: &DiagnosticBag, file_count: usize) -> String {
    let files = plural(file_count, "file", "files");
    if diagnostics.is_empty() {
        return format!("plat check: no problems found in {files}");
    }
    format!(
        "plat check: {}, {} in {files}",
        plural(diagnostics.error_count(), "error", "errors"),
        plural(diagnostics.warning_count(), "warning", "warnings"),
    )
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
