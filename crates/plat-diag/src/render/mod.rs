//! # Renderers: one set of facts, two views
//!
//! Both renderers take the same inputs (diagnostics plus the `SourceMap` that
//! resolves their spans) and differ only in the audience:
//!
//! - `json`: the `platinum.diagnostics/v1` document for tools and agents.
//! - `human`: colored, source-annotated reports for people, via `ariadne`.
//!
//! If a fact appears in human output, it must also be in the JSON.

pub mod human;
pub mod json;

#[cfg(test)]
mod tests;

pub use human::{HumanOptions, render_human};
pub use json::{SCHEMA, render_json};
