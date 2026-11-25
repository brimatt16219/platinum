//! # The Diagnostic type
//!
//! A `Diagnostic` is one message from the compiler: an error, a warning, or a
//! note. It is plain data. Nothing here knows how to print itself; the JSON
//! and human renderers (step 5) read these structs and produce output. That
//! split is the "diagnostics as data" rule from CLAUDE.md: the JSON form is the
//! source of truth, and pretty terminal output is only one view of it.
//!
//! The fields mirror the v1 JSON schema in `spec/09-tooling.md`:
//!
//! ```text
//! Diagnostic                         JSON
//! ----------                         ----
//! code: Code                         "code": "PT3001"
//! severity: Severity                 "severity": "error"
//! message: String                    "message": "refinement may not hold"
//! primary: Label                     "primary": { file, start, end, label }
//! secondary: Vec<Label>              "secondary": [ ... ]
//! notes: Vec<String>                 "notes": [ ... ]
//! fixes: Vec<Fix>                    "fixes": [ { id, title, applicability, edits } ]
//!                                    "docs": "plat explain PT3001"   (derived from code)
//! ```
//!
//! Things the renderer fills in rather than storing here:
//! - line and column numbers (computed from byte offsets via `SourceMap`),
//! - file names (looked up from each span's `FileId`),
//! - fix ids like `fix-1` (assigned in output order),
//! - the `docs` string (always `plat explain <code>`).
//!
//! The schema's `counterexample` field belongs to verification and will be
//! added in M4, when there is something to put in it.
//!
//! ## Building diagnostics
//!
//! Construction uses a small builder style so call sites read like the
//! message they produce:
//!
//! ```
//! use plat_diag::{Code, Diagnostic, FileId, Span};
//!
//! let file = FileId(0);
//! let d = Diagnostic::error(Code(2), "unterminated string literal", Span::new(file, 10, 25))
//!     .with_primary_label("this string is never closed")
//!     .with_note("strings cannot span lines unless they use triple quotes");
//! assert!(d.is_error());
//! ```

use std::fmt;

use crate::codes::Code;
use crate::span::Span;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// Compilation cannot succeed. `plat` exits with code 1.
    Error,
    /// Suspicious but allowed (lints such as `PT1041`).
    Warning,
    /// Extra information that is not itself a problem.
    Note,
}

impl Severity {
    /// The lowercase name used in JSON and human output.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A span with an optional short explanation, such as
/// "this condition was false". The label text may be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

impl Label {
    pub fn new(span: Span, message: impl Into<String>) -> Label {
        Label { span, message: message.into() }
    }
}

/// How safe it is to apply a fix without a human looking at it.
/// The three levels are borrowed from rustc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Applicability {
    /// Definitely correct. `plat fix --apply` and agents may apply it blindly.
    MachineApplicable,
    /// Plausible but may change meaning. Show it, let someone decide.
    MaybeIncorrect,
    /// Contains `<...>` placeholders that must be filled in by hand.
    HasPlaceholders,
}

impl Applicability {
    /// The kebab-case name used in JSON (`machine-applicable`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            Applicability::MachineApplicable => "machine-applicable",
            Applicability::MaybeIncorrect => "maybe-incorrect",
            Applicability::HasPlaceholders => "has-placeholders",
        }
    }
}

/// One text replacement: replace the bytes covered by `span` with
/// `replacement`. An empty span inserts; an empty replacement deletes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub replacement: String,
}

impl Edit {
    pub fn new(span: Span, replacement: impl Into<String>) -> Edit {
        Edit { span, replacement: replacement.into() }
    }
}

/// A suggested change that resolves (or might resolve) a diagnostic.
/// A fix may need several edits, possibly in different files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub title: String,
    pub applicability: Applicability,
    pub edits: Vec<Edit>,
}

impl Fix {
    pub fn new(title: impl Into<String>, applicability: Applicability, edits: Vec<Edit>) -> Fix {
        Fix { title: title.into(), applicability, edits }
    }
}

/// One compiler message. See the module docs for how fields map to JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub message: String,
    pub primary: Label,
    pub secondary: Vec<Label>,
    pub notes: Vec<String>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    /// A diagnostic with the given severity and a primary span with no label
    /// text yet. Most code uses the `error` or `warning` shorthands.
    pub fn new(severity: Severity, code: Code, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic {
            code,
            severity,
            message: message.into(),
            primary: Label::new(span, ""),
            secondary: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
        }
    }

    pub fn error(code: Code, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(Severity::Error, code, message, span)
    }

    pub fn warning(code: Code, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(Severity::Warning, code, message, span)
    }

    /// Sets the text shown under the primary span.
    #[must_use]
    pub fn with_primary_label(mut self, message: impl Into<String>) -> Diagnostic {
        self.primary.message = message.into();
        self
    }

    /// Adds a secondary span, for related locations
    /// ("the earlier definition is here").
    #[must_use]
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.secondary.push(Label::new(span, message));
        self
    }

    /// Adds a free-text note shown after the source snippet.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.notes.push(note.into());
        self
    }

    /// Adds a suggested fix.
    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Diagnostic {
        self.fixes.push(fix);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// The `docs` string for JSON output.
    pub fn docs(&self) -> String {
        format!("plat explain {}", self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::FileId;

    fn span(start: u32, end: u32) -> Span {
        Span::new(FileId(0), start, end)
    }

    #[test]
    fn builder_fills_every_field() {
        let fix = Fix::new(
            "include FEE in the balance check",
            Applicability::MaybeIncorrect,
            vec![Edit::new(span(360, 363), "amt + FEE")],
        );
        let d = Diagnostic::error(Code(3001), "refinement may not hold", span(412, 419))
            .with_primary_label("field `balance` requires `self >= 0`")
            .with_label(span(360, 378), "this condition was false")
            .with_note("balance would be 5 - 5 - 1 = -1")
            .with_fix(fix.clone());

        assert_eq!(d.code, Code(3001));
        assert!(d.is_error());
        assert_eq!(d.primary, Label::new(span(412, 419), "field `balance` requires `self >= 0`"));
        assert_eq!(d.secondary, vec![Label::new(span(360, 378), "this condition was false")]);
        assert_eq!(d.notes, vec!["balance would be 5 - 5 - 1 = -1".to_string()]);
        assert_eq!(d.fixes, vec![fix]);
        assert_eq!(d.docs(), "plat explain PT3001");
    }

    #[test]
    fn primary_label_defaults_to_empty() {
        let d = Diagnostic::warning(Code(1041), "ambiguous `?`", span(0, 5));
        assert_eq!(d.primary.message, "");
        assert!(!d.is_error());
    }

    #[test]
    fn names_match_json_schema() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Note.as_str(), "note");
        assert_eq!(Applicability::MachineApplicable.as_str(), "machine-applicable");
        assert_eq!(Applicability::MaybeIncorrect.as_str(), "maybe-incorrect");
        assert_eq!(Applicability::HasPlaceholders.as_str(), "has-placeholders");
    }
}
