//! # plat-diag: diagnostics as data
//!
//! Every error, warning, or note the Platinum compiler produces is a
//! `Diagnostic` value: a stable code (like `PT0001`), a severity, a message,
//! one or more labeled source spans, and optionally a machine-applicable fix.
//!
//! The key idea is that a diagnostic is **data first, text second**. The JSON
//! form (schema `platinum.diagnostics/v1`, see `spec/09-tooling.md`) is the
//! source of truth that agents and editors consume. The pretty terminal output
//! is just one renderer over the same struct. Keeping these separate means we
//! never parse our own error messages, and the wording can improve without
//! breaking any tool that depends on the code.
//!
//! This crate sits at the bottom of the dependency graph: every compiler phase
//! depends on it, and it depends on no other Platinum crate.
//!
//! Contents arrive over M0 (see `notes/M0-plan.md`):
//! - step 2: `Span` and `SourceFile` (byte offsets to line/column)
//! - step 3: `Diagnostic`, `Label`, `Fix`, `DiagnosticBag`
//! - step 4: the codes registry used by `plat explain`
//! - step 5: JSON and human (ariadne) renderers
