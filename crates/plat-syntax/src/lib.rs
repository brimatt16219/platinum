//! # plat-syntax: from source text to syntax tree
//!
//! This crate is the front of the compiler pipeline:
//!
//! ```text
//! source text --[lexer]--> tokens --[parser]--> AST
//! ```
//!
//! - The **lexer** (M0) reads characters and groups them into tokens: keywords,
//!   identifiers, literals, operators. Each token records its `Span` (where in
//!   the file it came from) but not its computed value; turning `1_000` into a
//!   number is a later concern. The lexer also decides which newlines end a
//!   statement (DECISIONS D-17) and splits interpolated strings into tokens
//!   (D-18).
//! - The **parser** (M1) turns tokens into an abstract syntax tree using
//!   recursive descent for statements and items, and Pratt parsing for
//!   expressions.
//!
//! Neither phase stops at the first mistake. Bad input becomes a diagnostic
//! (see `plat-diag`) and processing continues, so one run can report many
//! errors.
