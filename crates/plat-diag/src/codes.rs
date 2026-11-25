//! # Diagnostic codes
//!
//! Every diagnostic has a stable code such as `PT0001`. The code, not the
//! message text, is the diagnostic's identity: messages may be reworded, but a
//! code's meaning is fixed forever and codes are never reused
//! (`spec/09-tooling.md`). Agents, tests, and `plat explain` all key on it.
//!
//! Ranges (DECISIONS D-20 and `spec/09-tooling.md`):
//!
//! | Range           | Area                                      |
//! |-----------------|-------------------------------------------|
//! | PT0001-PT0099   | lexing                                    |
//! | PT0100-PT0999   | parsing                                   |
//! | PT1000-PT1999   | names, types, traits, patterns, mutation  |
//! | PT2000-PT2999   | effects, capabilities, concurrency        |
//! | PT3000-PT3999   | verification                              |
//! | PT4000-PT4999   | runtime (panic reports)                   |
//! | PT9000-PT9999   | internal compiler errors                  |
//!
//! Step 3 defines only the `Code` type. Step 4 adds the registry: one entry
//! per code with a title and the long explanation used by `plat explain`.

use std::fmt;

/// A diagnostic code. `Code(1)` displays as `PT0001`.
///
/// Stored as a number rather than a string so it is cheap to copy and compare,
/// and so formatting is always consistent (four digits, `PT` prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Code(pub u16);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PT{:04}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_with_prefix_and_padding() {
        assert_eq!(Code(1).to_string(), "PT0001");
        assert_eq!(Code(110).to_string(), "PT0110");
        assert_eq!(Code(3001).to_string(), "PT3001");
    }
}
