//! # DiagnosticBag: collect, don't stop
//!
//! A compiler that quits at the first error is painful: fix one typo, rerun,
//! find the next. Platinum's phases instead *collect* diagnostics and keep
//! going (the parser recovers from syntax errors, the type checker marks a bad
//! expression as an error type and moves on). A `DiagnosticBag` is the
//! collection each phase pushes into.
//!
//! The pattern in a phase looks like:
//!
//! ```
//! use plat_diag::{Code, Diagnostic, DiagnosticBag, FileId, Span};
//!
//! let mut diags = DiagnosticBag::new();
//! diags.push(Diagnostic::error(Code(1), "unexpected character `$`", Span::new(FileId(0), 4, 5)));
//! // ... keep lexing ...
//! if diags.has_errors() {
//!     // the CLI exits with code 1 after rendering them all
//! }
//! ```
//!
//! Output order matters for golden tests and for humans reading top to
//! bottom, so `sort` orders diagnostics by source position. The sort is
//! stable: two diagnostics at the same position keep the order they were
//! reported in.

use crate::diagnostic::{Diagnostic, Severity};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> DiagnosticBag {
        DiagnosticBag::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Moves every diagnostic from `other` into this bag (used when one phase
    /// hands its results to the next).
    pub fn append(&mut self, other: &mut DiagnosticBag) {
        self.diagnostics.append(&mut other.diagnostics);
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    pub fn error_count(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warning_count(&self) -> usize {
        self.count(Severity::Warning)
    }

    fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.diagnostics.iter()
    }

    /// Orders diagnostics by file, then start offset, then end offset.
    /// Stable, so ties keep their reporting order.
    pub fn sort(&mut self) {
        self.diagnostics.sort_by_key(|d| {
            (
                d.primary.span.file,
                d.primary.span.start,
                d.primary.span.end,
            )
        });
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl Extend<Diagnostic> for DiagnosticBag {
    fn extend<I: IntoIterator<Item = Diagnostic>>(&mut self, iter: I) {
        self.diagnostics.extend(iter);
    }
}

impl<'a> IntoIterator for &'a DiagnosticBag {
    type Item = &'a Diagnostic;
    type IntoIter = std::slice::Iter<'a, Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for DiagnosticBag {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::Code;
    use crate::span::{FileId, Span};

    fn err(file: u32, start: u32, msg: &str) -> Diagnostic {
        Diagnostic::error(Code(1), msg, Span::new(FileId(file), start, start + 1))
    }

    fn warn(start: u32) -> Diagnostic {
        Diagnostic::warning(Code(1041), "w", Span::new(FileId(0), start, start + 1))
    }

    #[test]
    fn empty_bag_has_no_errors() {
        let bag = DiagnosticBag::new();
        assert!(bag.is_empty());
        assert!(!bag.has_errors());
        assert_eq!(bag.error_count(), 0);
    }

    #[test]
    fn warnings_alone_are_not_errors() {
        let mut bag = DiagnosticBag::new();
        bag.push(warn(0));
        bag.push(warn(5));
        assert!(!bag.has_errors());
        assert_eq!(bag.warning_count(), 2);
    }

    #[test]
    fn counts_by_severity() {
        let mut bag = DiagnosticBag::new();
        bag.push(err(0, 0, "a"));
        bag.push(warn(3));
        bag.push(err(0, 9, "b"));
        assert!(bag.has_errors());
        assert_eq!(bag.len(), 3);
        assert_eq!(bag.error_count(), 2);
        assert_eq!(bag.warning_count(), 1);
    }

    #[test]
    fn sort_orders_by_file_then_position_and_is_stable() {
        let mut bag = DiagnosticBag::new();
        bag.push(err(1, 0, "file1"));
        bag.push(err(0, 20, "late"));
        bag.push(err(0, 5, "tie-first"));
        bag.push(err(0, 5, "tie-second"));
        bag.sort();
        let order: Vec<&str> = bag.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(order, ["tie-first", "tie-second", "late", "file1"]);
    }

    #[test]
    fn append_moves_everything() {
        let mut a = DiagnosticBag::new();
        let mut b = DiagnosticBag::new();
        a.push(err(0, 0, "a"));
        b.push(err(0, 1, "b"));
        a.append(&mut b);
        assert_eq!(a.len(), 2);
        assert!(b.is_empty());
    }
}
