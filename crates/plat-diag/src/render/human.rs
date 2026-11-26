//! # Human renderer: pretty terminal output via `ariadne`
//!
//! Produces the familiar compiler-style report: a header line with severity
//! and code, the source lines involved, underlines with labels, then notes and
//! help. It reads exactly the same `Diagnostic` structs as the JSON renderer;
//! nothing here may invent information the JSON does not carry.
//!
//! We use the `ariadne` crate for the drawing. Our job is only to translate:
//!
//! | Platinum                   | ariadne                                   |
//! |----------------------------|-------------------------------------------|
//! | `Severity::Error/Warning`  | `ReportKind::Error/Warning`               |
//! | `Severity::Note`           | `ReportKind::Advice`                      |
//! | `code`                     | `.with_code(...)`, shown as `[PT0002]`    |
//! | `primary`, `secondary`     | `Label`s, primary first                   |
//! | `notes`                    | `.with_note(...)`                         |
//! | `fixes`                    | `.with_help(...)`, one per fix            |
//!
//! Two details matter:
//!
//! - **Byte offsets.** ariadne counts characters by default, but our spans
//!   are byte offsets. `IndexType::Byte` tells it to interpret ranges as
//!   bytes, so `€` and emoji do not shift the underlines.
//! - **Color is optional.** Terminals get color; tests and pipes get plain
//!   text (`HumanOptions { color: false }`) so snapshots are readable and
//!   stable.
//!
//! Two ariadne behaviors we work around (found by reading its source):
//!
//! - It only draws an underline for a label that has text. A label with an
//!   empty message would be invisible without color, so an empty primary
//!   label shows the diagnostic's message, and an empty secondary label shows
//!   "related location".
//! - It starts a new snippet block whenever a label's line is *before* the
//!   previous label's line. We hand it labels sorted by position, so labels in
//!   one file share one snippet. (Which label is primary is still conveyed by
//!   the report header, which points at the primary span.)

use std::ops::Range;

use ariadne::{Config, IndexType, Label as AriadneLabel, Report, ReportKind};

use crate::diagnostic::{Diagnostic, Severity};
use crate::source::SourceMap;
use crate::span::Span;

/// Rendering options for human output.
#[derive(Debug, Clone, Copy)]
pub struct HumanOptions {
    /// Use ANSI colors. The CLI turns this off when output is not a terminal.
    pub color: bool,
}

impl Default for HumanOptions {
    fn default() -> HumanOptions {
        HumanOptions { color: true }
    }
}

/// ariadne identifies a span by (source id, range). We use the file name as
/// the id so it is printed in the report header.
type AriadneSpan = (String, Range<usize>);

/// Renders every diagnostic, separated by blank lines, into one string.
pub fn render_human<'a>(
    diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
    sources: &SourceMap,
    options: HumanOptions,
) -> String {
    let rendered: Vec<String> = diagnostics
        .into_iter()
        .map(|d| render_one(d, sources, options))
        .collect();
    rendered.join("\n")
}

/// Renders a single diagnostic.
pub fn render_one(diagnostic: &Diagnostic, sources: &SourceMap, options: HumanOptions) -> String {
    let config = Config::default()
        .with_color(options.color)
        .with_index_type(IndexType::Byte);

    let kind = match diagnostic.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Note => ReportKind::Advice,
    };

    let mut builder = Report::build(kind, ariadne_span(diagnostic.primary.span, sources))
        .with_config(config)
        .with_code(diagnostic.code)
        .with_message(&diagnostic.message);

    // Collect (span, text) for every label, fill in empty texts, then sort by
    // position so ariadne keeps each file's labels in one snippet.
    let primary_text = non_empty_or(&diagnostic.primary.message, &diagnostic.message);
    let mut labels: Vec<(Span, &str)> = vec![(diagnostic.primary.span, primary_text)];
    labels.extend(
        diagnostic
            .secondary
            .iter()
            .map(|l| (l.span, non_empty_or(&l.message, "related location"))),
    );
    // Stable sort: labels at the same position keep primary-first order.
    labels.sort_by_key(|(span, _)| (span.file, span.start, span.end));
    for (span, text) in labels {
        builder = builder.with_label(AriadneLabel::new(ariadne_span(span, sources)).with_message(text));
    }
    for note in &diagnostic.notes {
        builder = builder.with_note(note);
    }
    for fix in &diagnostic.fixes {
        builder = builder.with_help(format!("{} ({})", fix.title, fix.applicability.as_str()));
    }
    builder = builder.with_note(format!("for more information, run `{}`", diagnostic.docs()));

    // ariadne reads source text through a "cache". We hand it every file in
    // the SourceMap, keyed by name, matching the ids used in our spans.
    let cache = ariadne::sources(
        sources
            .iter()
            .map(|(_, file)| (file.name().to_string(), file.text().to_string())),
    );

    let mut out = Vec::new();
    match builder.finish().write(cache, &mut out) {
        Ok(()) => String::from_utf8_lossy(&out).into_owned(),
        // Only happens if a span names a file missing from the SourceMap
        // (a compiler bug). Fall back to a one-line form rather than losing
        // the diagnostic.
        Err(_) => fallback(diagnostic),
    }
}

fn non_empty_or<'a>(text: &'a str, fallback: &'a str) -> &'a str {
    if text.is_empty() { fallback } else { text }
}

fn ariadne_span(span: Span, sources: &SourceMap) -> AriadneSpan {
    let name = sources
        .get(span.file)
        .map_or_else(|| "<unknown>".to_string(), |f| f.name().to_string());
    (name, span.start as usize..span.end as usize)
}

fn fallback(d: &Diagnostic) -> String {
    format!(
        "{}[{}]: {} (at bytes {}..{})\n",
        d.severity, d.code, d.message, d.primary.span.start, d.primary.span.end
    )
}
