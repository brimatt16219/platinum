//! # Diagnostic codes and the registry behind `plat explain`
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
//! ## How the registry works
//!
//! Each code has two parts here:
//!
//! 1. A named constant (`UNTERMINATED_STRING`) that compiler code uses when it
//!    reports the problem. Using a name instead of `Code(2)` at call sites
//!    means a typo is a compile error, and searching for the name finds every
//!    place the diagnostic is produced.
//! 2. A `CodeInfo` entry in `REGISTRY` with a short title and a long
//!    explanation. `plat explain PT0002` prints that explanation.
//!
//! The tests at the bottom keep the two in sync: every code is unique, sits in
//! a known range, appears in ascending order, and has a non-empty title and
//! explanation.
//!
//! **Adding a code:** add the constant, add the `CodeInfo` entry in numeric
//! order, and never renumber or delete an existing one. A retired code stays
//! in the registry with its explanation saying it is no longer emitted.

use std::fmt;
use std::str::FromStr;

/// A diagnostic code. `Code(1)` displays as `PT0001`.
///
/// Stored as a number rather than a string so it is cheap to copy and compare,
/// and so formatting is always consistent (four digits, `PT` prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Code(pub u16);

impl Code {
    /// The compiler area this code belongs to, from the range table above.
    /// `None` for numbers outside every range (such as `PT5000`).
    pub fn area(self) -> Option<&'static str> {
        match self.0 {
            1..=99 => Some("lexing"),
            100..=999 => Some("parsing"),
            1000..=1999 => Some("names, types, traits, patterns, mutation"),
            2000..=2999 => Some("effects, capabilities, concurrency"),
            3000..=3999 => Some("verification"),
            4000..=4999 => Some("runtime"),
            9000..=9999 => Some("internal compiler error"),
            _ => None,
        }
    }

    /// The registry entry for this code, if it has one.
    pub fn info(self) -> Option<&'static CodeInfo> {
        lookup(self)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PT{:04}", self.0)
    }
}

/// Why a string could not be read as a `Code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCodeError {
    pub input: String,
}

impl fmt::Display for ParseCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not a diagnostic code; expected the form PT0001",
            self.input
        )
    }
}

impl std::error::Error for ParseCodeError {}

/// Parses `PT0001` (case-insensitive: `pt0001` works too). Exactly four
/// digits are required so that `PT1` and `PT0001` cannot both mean the same
/// code in different places.
impl FromStr for Code {
    type Err = ParseCodeError;

    fn from_str(s: &str) -> Result<Code, ParseCodeError> {
        let error = || ParseCodeError {
            input: s.to_string(),
        };
        let trimmed = s.trim();
        let prefix = trimmed.get(..2).ok_or_else(error)?;
        let digits = trimmed.get(2..).ok_or_else(error)?;
        if !prefix.eq_ignore_ascii_case("PT")
            || digits.len() != 4
            || !digits.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(error());
        }
        digits.parse::<u16>().map(Code).map_err(|_| error())
    }
}

/// Everything `plat explain` knows about one code.
#[derive(Debug)]
pub struct CodeInfo {
    pub code: Code,
    /// A short phrase, such as "unterminated string literal".
    pub title: &'static str,
    /// One or more paragraphs: what the problem is, why Platinum treats it
    /// this way, and an example of wrong and corrected code.
    pub explanation: &'static str,
}

/// Finds the registry entry for `code`. The registry is sorted (a test
/// enforces it), so this is a binary search.
pub fn lookup(code: Code) -> Option<&'static CodeInfo> {
    REGISTRY
        .binary_search_by_key(&code, |info| info.code)
        .ok()
        .map(|index| &REGISTRY[index])
}

// ---------------------------------------------------------------------------
// Lexing: PT0001-PT0099
// ---------------------------------------------------------------------------

pub const UNEXPECTED_CHARACTER: Code = Code(1);
pub const UNTERMINATED_STRING: Code = Code(2);
pub const UNTERMINATED_BLOCK_COMMENT: Code = Code(3);
pub const INVALID_ESCAPE: Code = Code(4);
pub const MALFORMED_NUMBER: Code = Code(5);
pub const UNKNOWN_NUMBER_SUFFIX: Code = Code(6);
pub const UNTERMINATED_INTERPOLATION: Code = Code(7);
pub const RESERVED_KEYWORD: Code = Code(8);

/// All known codes, in ascending order.
pub static REGISTRY: &[CodeInfo] = &[
    CodeInfo {
        code: UNEXPECTED_CHARACTER,
        title: "unexpected character",
        explanation: r#"The lexer found a character that cannot start any Platinum token.

Platinum source is UTF-8, and non-ASCII characters are allowed inside string
literals and comments. Outside of those, only the characters used by
identifiers, numbers, operators, and punctuation are valid. Common causes are a
stray symbol such as `$` or `@`, or "smart quotes" pasted from a word
processor instead of plain `"`.

Example:

    let price = $5        // error: `$` cannot start a token

Fix:

    let price = 5

The lexer reports the character and continues with the next one, so one
mistake does not hide later ones."#,
    },
    CodeInfo {
        code: UNTERMINATED_STRING,
        title: "unterminated string literal",
        // Two hashes (`r##"..."##`) because the text contains `"#`, which would
        // end an `r#"..."#` raw string early. Platinum raw strings work the same way.
        explanation: r##"A string literal was opened with `"` but never closed.

Ordinary string literals must end on the same line they start on. For text
that spans several lines, use a triple-quoted string (`"""` ... `"""`), which
also strips the common leading indentation.

Example:

    puts("hello)          // error: the string runs to the end of the line

Fix:

    puts("hello")

If the string really should contain a `"`, escape it as `\"`, or use a raw
string with hashes: r#"she said "hi""#"##,
    },
    CodeInfo {
        code: UNTERMINATED_BLOCK_COMMENT,
        title: "unterminated block comment",
        explanation: r#"A block comment opened with `/*` was never closed with `*/`.

Block comments in Platinum nest, so every `/*` inside a comment needs its own
matching `*/`. This makes it safe to comment out code that already contains
block comments, but it also means a stray `/*` inside a comment must be
closed.

Example:

    /* outer /* inner */      // error: the outer comment is still open

Fix:

    /* outer /* inner */ */

The diagnostic points at the `/*` that was never closed."#,
    },
    CodeInfo {
        code: INVALID_ESCAPE,
        title: "invalid escape sequence",
        explanation: r#"A backslash in a string literal is followed by a character that does not form
a valid escape.

The valid escapes are:

    \n  newline           \t  tab              \r  carriage return
    \\  backslash         \"  double quote     \#  literal `#` (no interpolation)
    \u{1F600}  a Unicode scalar value, 1 to 6 hex digits

Example:

    let path = "C:\new\folder"   // error: `\f` is not an escape

Fix, either escape the backslashes or use a raw string, which has no escapes:

    let path = "C:\\new\\folder"
    let path = r"C:\new\folder""#,
    },
    CodeInfo {
        code: MALFORMED_NUMBER,
        title: "malformed number literal",
        explanation: r#"A number literal is not well formed.

Platinum accepts decimal integers (`42`, `1_000_000`), hexadecimal (`0xFF`),
binary (`0b1010`), decimals (`1.50`, an exact `Decimal`), and floats with an
`f` suffix (`1.5f`, `2.0e-3f`). Underscores may separate digits for
readability.

Examples:

    0x        // error: no digits after the base prefix
    0b102       // error: `2` is not a binary digit

Fix:

    0xFF
    0b101"#,
    },
    CodeInfo {
        code: UNKNOWN_NUMBER_SUFFIX,
        title: "unknown numeric suffix",
        explanation: r#"A number literal ends with a suffix Platinum does not recognize.

Unsuffixed integers are arbitrary-precision `Int`, and unsuffixed numbers with
a decimal point are exact `Decimal`. A suffix selects a different type:

    i8  i16  i32  i64     fixed-width signed integers
    u8  u16  u32  u64     fixed-width unsigned integers
    f                     Float (IEEE 754 double precision)

Example:

    let small = 42i7      // error: there is no 7-bit integer type

Fix:

    let small = 42i8

There is no `f32` or `f64` suffix: `Float` is always 64-bit, written `1.5f`."#,
    },
    CodeInfo {
        code: UNTERMINATED_INTERPOLATION,
        title: "unterminated string interpolation",
        explanation: r#"An interpolation `#{` inside a string was never closed with `}`.

String interpolation embeds an expression in a string: `"total: #{n}"`. The
expression runs from `#{` to the matching `}`, and the string must continue
after it until its closing `"`.

Example:

    puts("total: #{n")    // error: `#{` is never closed

Fix:

    puts("total: #{n}")

To write a literal `#{` in a string without interpolating, escape the hash:
"\#{not interpolated}"."#,
    },
    CodeInfo {
        code: RESERVED_KEYWORD,
        title: "use of reserved keyword",
        explanation: r#"A word that Platinum reserves for future use appears as an identifier.

These words are not keywords yet, but are reserved so that adding them later
does not break existing programs:

    async  decreases  macro  unsafe  yield

Example:

    let yield = 3         // error: `yield` is reserved

Fix, choose another name:

    let output = 3"#,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_with_prefix_and_padding() {
        assert_eq!(Code(1).to_string(), "PT0001");
        assert_eq!(Code(110).to_string(), "PT0110");
        assert_eq!(Code(3001).to_string(), "PT3001");
    }

    #[test]
    fn parses_canonical_and_lowercase() {
        assert_eq!("PT0001".parse::<Code>(), Ok(Code(1)));
        assert_eq!("pt3001".parse::<Code>(), Ok(Code(3001)));
        assert_eq!(" PT0110 ".parse::<Code>(), Ok(Code(110)));
    }

    #[test]
    fn rejects_malformed_codes() {
        for bad in [
            "", "PT", "PT1", "PT00001", "XX0001", "PT00a1", "0001", "PT-001", "€T0001",
        ] {
            assert!(bad.parse::<Code>().is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn display_and_parse_round_trip() {
        for info in REGISTRY {
            assert_eq!(info.code.to_string().parse::<Code>(), Ok(info.code));
        }
    }

    #[test]
    fn areas_follow_the_range_table() {
        assert_eq!(Code(1).area(), Some("lexing"));
        assert_eq!(Code(99).area(), Some("lexing"));
        assert_eq!(Code(100).area(), Some("parsing"));
        assert_eq!(
            Code(2001).area(),
            Some("effects, capabilities, concurrency")
        );
        assert_eq!(Code(0).area(), None);
        assert_eq!(Code(5000).area(), None);
    }

    #[test]
    fn registry_is_strictly_ascending_so_codes_are_unique() {
        for pair in REGISTRY.windows(2) {
            assert!(
                pair[0].code < pair[1].code,
                "{} must come before {} and appear only once",
                pair[0].code,
                pair[1].code
            );
        }
    }

    #[test]
    fn every_code_is_in_a_known_range() {
        for info in REGISTRY {
            assert!(
                info.code.area().is_some(),
                "{} is outside every range",
                info.code
            );
        }
    }

    #[test]
    fn every_entry_is_documented() {
        for info in REGISTRY {
            assert!(!info.title.trim().is_empty(), "{} has no title", info.code);
            assert!(
                !info.explanation.trim().is_empty(),
                "{} has no explanation",
                info.code
            );
            assert!(
                info.explanation.contains("Example"),
                "{} explanation should include an example",
                info.code
            );
        }
    }

    #[test]
    fn explanations_follow_the_writing_style() {
        // CLAUDE.md: no em-dashes in docs. `plat explain` output is docs.
        for info in REGISTRY {
            assert!(
                !info.title.contains('\u{2014}'),
                "{} title has an em-dash",
                info.code
            );
            assert!(
                !info.explanation.contains('\u{2014}'),
                "{} explanation has an em-dash",
                info.code
            );
        }
    }

    #[test]
    fn lookup_finds_every_entry_and_nothing_else() {
        for info in REGISTRY {
            assert_eq!(lookup(info.code).map(|i| i.code), Some(info.code));
        }
        assert!(lookup(Code(98)).is_none());
        assert_eq!(
            UNTERMINATED_STRING.info().map(|i| i.title),
            Some("unterminated string literal")
        );
    }
}
