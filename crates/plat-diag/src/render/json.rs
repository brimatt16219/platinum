//! # JSON renderer: the machine-readable source of truth
//!
//! Turns diagnostics into the `platinum.diagnostics/v1` document described in
//! `spec/09-tooling.md`. This is what `plat check --json`, the MCP server, and
//! agents consume, so its shape is a public contract: fields may be added in
//! later versions, but existing ones keep their names and meaning.
//!
//! ## Why separate "output" structs?
//!
//! The `Diagnostic` type stores what the compiler knows: byte spans and file
//! ids. The JSON needs more: line and column numbers, file names, fix ids. So
//! this module defines small private structs (`JsonDiagnostic`, `JsonLabel`,
//! and so on) whose layout *is* the JSON layout, fills them in from a
//! `Diagnostic` plus the `SourceMap`, and lets `serde` turn them into text.
//!
//! This is a common pattern (often called DTOs, data transfer objects, in C#
//! and Java): the core types stay free of serialization details, and the wire
//! format is visible in one place. `#[derive(Serialize)]` generates the code
//! that walks each struct and writes its fields as JSON keys, in declaration
//! order, which keeps the output stable for snapshot tests.
//!
//! ## Lifetimes in these structs
//!
//! `JsonLabel<'a>` borrows strings (`&'a str`) from the diagnostics and source
//! files instead of copying them. The `'a` tells the compiler "these borrowed
//! strings live at least as long as the `JsonLabel`". The output structs are
//! built, serialized, and dropped inside `render_json`, so the borrows never
//! outlive their data.

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Edit, Fix, Label, Severity};
use crate::source::SourceMap;
use crate::span::Span;

/// The schema identifier written into every document.
pub const SCHEMA: &str = "platinum.diagnostics/v1";

/// Shown as the file name if a span refers to a file missing from the
/// `SourceMap`. That would be a compiler bug, but rendering must not crash.
const UNKNOWN_FILE: &str = "<unknown>";

/// Renders a complete JSON document for one tool run.
///
/// `tool` names the command that produced the diagnostics (`"plat check"`).
/// Diagnostics are written in the order given; callers sort the bag first
/// (`DiagnosticBag::sort`) when they want source order.
pub fn render_json<'a>(
    tool: &str,
    diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
    sources: &SourceMap,
) -> String {
    let diagnostics: Vec<&Diagnostic> = diagnostics.into_iter().collect();

    let document = JsonDocument {
        schema: SCHEMA,
        tool,
        summary: JsonSummary {
            errors: count(&diagnostics, Severity::Error),
            warnings: count(&diagnostics, Severity::Warning),
        },
        diagnostics: diagnostics.iter().map(|d| JsonDiagnostic::new(d, sources)).collect(),
    };

    // Serializing plain structs of strings and numbers cannot fail: there are
    // no maps with non-string keys and no custom serializers that return
    // errors. This is a compiler invariant, not user data.
    serde_json::to_string_pretty(&document).expect("diagnostic JSON is always serializable")
}

fn count(diagnostics: &[&Diagnostic], severity: Severity) -> usize {
    diagnostics.iter().filter(|d| d.severity == severity).count()
}

// --- Output structs: their fields, in order, are the JSON layout. -----------

#[derive(Serialize)]
struct JsonDocument<'a> {
    schema: &'static str,
    tool: &'a str,
    summary: JsonSummary,
    diagnostics: Vec<JsonDiagnostic<'a>>,
}

/// `proven`, `refuted`, and `unknown` counts join this in M4 (`plat prove`).
#[derive(Serialize)]
struct JsonSummary {
    errors: usize,
    warnings: usize,
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    code: String,
    severity: &'static str,
    message: &'a str,
    primary: JsonLabel<'a>,
    secondary: Vec<JsonLabel<'a>>,
    notes: &'a [String],
    fixes: Vec<JsonFix<'a>>,
    docs: String,
}

impl<'a> JsonDiagnostic<'a> {
    fn new(d: &'a Diagnostic, sources: &'a SourceMap) -> JsonDiagnostic<'a> {
        JsonDiagnostic {
            code: d.code.to_string(),
            severity: d.severity.as_str(),
            message: &d.message,
            primary: JsonLabel::new(&d.primary, sources),
            secondary: d.secondary.iter().map(|l| JsonLabel::new(l, sources)).collect(),
            notes: &d.notes,
            // Fix ids are numbered per diagnostic, starting at 1.
            fixes: d
                .fixes
                .iter()
                .enumerate()
                .map(|(i, f)| JsonFix::new(i + 1, f, sources))
                .collect(),
            docs: d.docs(),
        }
    }
}

#[derive(Serialize)]
struct JsonLabel<'a> {
    file: &'a str,
    start: JsonPosition,
    end: JsonPosition,
    label: &'a str,
}

impl<'a> JsonLabel<'a> {
    fn new(label: &'a Label, sources: &'a SourceMap) -> JsonLabel<'a> {
        JsonLabel {
            file: file_name(label.span, sources),
            start: JsonPosition::new(label.span, label.span.start, sources),
            end: JsonPosition::new(label.span, label.span.end, sources),
            label: &label.message,
        }
    }
}

/// A point in a file, given three ways (DECISIONS D-19): 1-based line,
/// 1-based character column, and 0-based byte offset.
#[derive(Serialize)]
struct JsonPosition {
    line: u32,
    col: u32,
    offset: u32,
}

impl JsonPosition {
    fn new(span: Span, offset: u32, sources: &SourceMap) -> JsonPosition {
        match sources.get(span.file) {
            Some(file) => {
                let lc = file.line_col(offset);
                JsonPosition { line: lc.line, col: lc.col, offset }
            }
            // Unknown file: no way to compute line/col. 0 is never a valid
            // 1-based value, so consumers can tell something is wrong.
            None => JsonPosition { line: 0, col: 0, offset },
        }
    }
}

#[derive(Serialize)]
struct JsonFix<'a> {
    id: String,
    title: &'a str,
    applicability: &'static str,
    edits: Vec<JsonEdit<'a>>,
}

impl<'a> JsonFix<'a> {
    fn new(number: usize, fix: &'a Fix, sources: &'a SourceMap) -> JsonFix<'a> {
        JsonFix {
            id: format!("fix-{number}"),
            title: &fix.title,
            applicability: fix.applicability.as_str(),
            edits: fix.edits.iter().map(|e| JsonEdit::new(e, sources)).collect(),
        }
    }
}

/// Edits use plain byte offsets: a tool applying them slices the file bytes
/// directly, so line/col would only add a chance of disagreement.
#[derive(Serialize)]
struct JsonEdit<'a> {
    file: &'a str,
    start: u32,
    end: u32,
    replacement: &'a str,
}

impl<'a> JsonEdit<'a> {
    fn new(edit: &'a Edit, sources: &'a SourceMap) -> JsonEdit<'a> {
        JsonEdit {
            file: file_name(edit.span, sources),
            start: edit.span.start,
            end: edit.span.end,
            replacement: &edit.replacement,
        }
    }
}

fn file_name(span: Span, sources: &SourceMap) -> &str {
    sources.get(span.file).map_or(UNKNOWN_FILE, |f| f.name())
}
