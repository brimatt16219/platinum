//! # Tokens: the lexer's output vocabulary
//!
//! The lexer reads characters and produces a flat list of **tokens**. A token
//! is a classification ("this is the keyword `fn`", "this is an integer
//! literal") plus the `Span` where it appeared. The exact characters (the
//! **lexeme**) are not copied into the token: they can always be recovered by
//! slicing the source with the span. That keeps tokens small (`Copy`, a few
//! bytes) and the lexer lossless.
//!
//! Example: `let total = 1_000` lexes to
//!
//! ```text
//! Kw(Let)     span 0..3
//! LowerIdent  span 4..9     (text "total", recovered from the span)
//! Eq          span 10..11
//! Int { suffix: None }  span 12..17  (value computed later, by the parser)
//! Eof         span 17..17
//! ```
//!
//! ## What the token kinds encode, and what they leave to later phases
//!
//! - **Identifiers** come in two shapes (`spec/01-lexical.md`): `LowerIdent`
//!   for values and functions, `UpperIdent` for types, constructors, and
//!   capabilities. A trailing `?` or `!` absorbed by the lexer is part of the
//!   identifier's text (`empty?`, `sort!`).
//! - **Number literals** record their *kind* and validated suffix (`42i64`
//!   is `Int { suffix: Some(I64) }`), but not their value. Turning `1_000`
//!   into a big integer needs `num-bigint`, which belongs to the parser (M1).
//! - **Strings** without interpolation are one `Str` token. With
//!   interpolation (DECISIONS D-18) they are split into pieces:
//!
//!   ```text
//!   "a #{x} b #{y} c"
//!   StrStart("a ")  InterpStart  x  InterpEnd
//!   StrMiddle(" b ")  InterpStart  y  InterpEnd  StrEnd(" c")
//!   ```
//!
//!   Escape sequences are validated by the lexer but decoded by the parser.
//! - **Keywords** are a separate `Keyword` enum wrapped in `TokenKind::Kw`,
//!   so "is this any keyword?" is one pattern match. `result` and `self` are
//!   lexed as keywords even though they are contextual; the parser decides
//!   what they mean in each position.
//! - **`Newline`** tokens only appear where a newline ends a statement
//!   (DECISIONS D-17). The two helper methods `can_end_statement` and
//!   `continues_line` encode that rule; the lexer (step 10) uses them.
//! - **`Error`** marks characters the lexer could not make sense of. A
//!   diagnostic has already been reported for it; the parser treats it as a
//!   placeholder and moves on.

use std::fmt;

use plat_diag::Span;

/// One token: what it is, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

/// Every kind of token Platinum has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // --- Names ---------------------------------------------------------
    /// `total`, `_`, `empty?`, `sort!`: values, functions, fields, modules.
    LowerIdent,
    /// `Money`, `Some`, `Gateway`: types, constructors, tags, traits, capabilities.
    UpperIdent,
    /// A keyword such as `fn` or `match`.
    Kw(Keyword),

    // --- Literals --------------------------------------------------------
    /// `42`, `0xFF`, `0b1010`, `1_000`, or sized: `42i64`, `255u8`.
    Int {
        suffix: Option<IntSuffix>,
    },
    /// `1.50`: an exact base-10 `Decimal` (no `f` suffix).
    Decimal,
    /// `1.5f`, `2.0e-3f`: an IEEE 754 `Float`.
    Float,
    /// A whole string with no interpolation: `"hello"` or `"""..."""`.
    Str {
        quote: Quote,
    },
    /// A raw string, never interpolated: `r"C:\x"`, `r#"has "quotes""#`.
    RawStr,
    /// Opening piece of an interpolated string, from the quote up to `#{`.
    StrStart {
        quote: Quote,
    },
    /// Text between `}` and the next `#{` in an interpolated string.
    StrMiddle {
        quote: Quote,
    },
    /// Closing piece of an interpolated string, from `}` to the closing quote.
    StrEnd {
        quote: Quote,
    },
    /// `#{` inside a string.
    InterpStart,
    /// The `}` that closes an interpolation.
    InterpEnd,

    // --- Comments --------------------------------------------------------
    /// `/// text`: kept as a token so the parser can attach it to the next
    /// item for `plat doc`. Ordinary `//` and `/* */` comments are skipped.
    DocComment,

    // --- Operators -------------------------------------------------------
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Percent,   // %
    EqEq,      // ==
    BangEq,    // !=
    Lt,        // <
    LtEq,      // <=
    Gt,        // >
    GtEq,      // >=
    AndAnd,    // &&
    OrOr,      // ||
    Bang,      // !
    Eq,        // =
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    Arrow,     // ->
    FatArrow,  // =>
    Pipe,      // |>
    Question,  // ?
    Dot,       // .
    DotDot,    // ..
    DotDotEq,  // ..=
    Colon,     // :
    Comma,     // ,
    Semicolon, // ;
    Bar,       // |

    // --- Delimiters ------------------------------------------------------
    LParen,      // (
    RParen,      // )
    LBracket,    // [
    RBracket,    // ]
    LBrace,      // {
    RBrace,      // }
    HashBracket, // #[  (start of an attribute)

    // --- Structure -------------------------------------------------------
    /// A newline that ends a statement (DECISIONS D-17).
    Newline,
    /// End of input. Always the last token, with an empty span.
    Eof,
    /// Characters that could not be lexed. A diagnostic was already reported.
    Error,
}

/// The suffix on a sized integer literal (`42i64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

impl IntSuffix {
    /// Parses the suffix text after the digits, such as `"i64"`.
    pub fn parse(s: &str) -> Option<IntSuffix> {
        match s {
            "i8" => Some(IntSuffix::I8),
            "i16" => Some(IntSuffix::I16),
            "i32" => Some(IntSuffix::I32),
            "i64" => Some(IntSuffix::I64),
            "u8" => Some(IntSuffix::U8),
            "u16" => Some(IntSuffix::U16),
            "u32" => Some(IntSuffix::U32),
            "u64" => Some(IntSuffix::U64),
            _ => None,
        }
    }

    /// The Platinum type this suffix selects (`I64`, `U8`, ...).
    pub fn type_name(self) -> &'static str {
        match self {
            IntSuffix::I8 => "I8",
            IntSuffix::I16 => "I16",
            IntSuffix::I32 => "I32",
            IntSuffix::I64 => "I64",
            IntSuffix::U8 => "U8",
            IntSuffix::U16 => "U16",
            IntSuffix::U32 => "U32",
            IntSuffix::U64 => "U64",
        }
    }
}

/// Which quoting a string piece belongs to. Triple-quoted strings get their
/// common leading indentation stripped, so the parser needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quote {
    /// `"..."`
    Single,
    /// `"""..."""`
    Triple,
}

/// Generates the `Keyword` enum together with its text table, so the two can
/// never drift apart. Each line is `Variant => "text"`.
///
/// This is the one macro in the lexer. It earns its place (CLAUDE.md: no
/// macros unless they remove real repetition) by replacing three hand-kept
/// lists: the enum, `as_str`, the lookup table, and `describe`.
macro_rules! keywords {
    ($($variant:ident => $text:literal,)*) => {
        /// A Platinum keyword (`spec/01-lexical.md`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Keyword {
            $($variant,)*
        }

        impl Keyword {
            /// Every keyword, in alphabetical order.
            pub const ALL: &'static [Keyword] = &[$(Keyword::$variant,)*];

            /// The keyword's source text.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Keyword::$variant => $text,)*
                }
            }

            /// Looks up a word. `None` for ordinary identifiers.
            pub fn lookup(word: &str) -> Option<Keyword> {
                match word {
                    $($text => Some(Keyword::$variant),)*
                    _ => None,
                }
            }

            /// The keyword in backticks for error messages, such as "`while`".
            /// `concat!` glues the pieces together at compile time.
            pub fn describe(self) -> &'static str {
                match self {
                    $(Keyword::$variant => concat!("`", $text, "`"),)*
                }
            }
        }
    };
}

keywords! {
    As => "as",
    Assert => "assert",
    Block => "block",
    Break => "break",
    Cap => "cap",
    Const => "const",
    Continue => "continue",
    Else => "else",
    Ensures => "ensures",
    False => "false",
    Fn => "fn",
    For => "for",
    Handle => "handle",
    If => "if",
    Impl => "impl",
    Import => "import",
    In => "in",
    Invariant => "invariant",
    Let => "let",
    Match => "match",
    Needs => "needs",
    Opaque => "opaque",
    Pub => "pub",
    Requires => "requires",
    Result => "result",
    Return => "return",
    Scope => "scope",
    SelfValue => "self",
    Spawn => "spawn",
    Test => "test",
    Trait => "trait",
    True => "true",
    Type => "type",
    Var => "var",
    Where => "where",
    While => "while",
    With => "with",
}

/// Words reserved for future use. Using one as an identifier is `PT0008`.
pub const RESERVED_WORDS: &[&str] = &["async", "decreases", "macro", "unsafe", "yield"];

/// True if `word` is reserved for future use.
pub fn is_reserved(word: &str) -> bool {
    RESERVED_WORDS.contains(&word)
}

impl TokenKind {
    /// Can a statement end right after this token? The first half of the
    /// newline rule (DECISIONS D-17): a newline after `x` or `)` may end a
    /// statement; a newline after `+` or `(` never does.
    pub fn can_end_statement(self) -> bool {
        use TokenKind::*;
        match self {
            LowerIdent | UpperIdent => true,
            Int { .. } | Decimal | Float | Str { .. } | RawStr | StrEnd { .. } => true,
            RParen | RBracket | RBrace | Question => true,
            Kw(kw) => matches!(
                kw,
                Keyword::Return
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::SelfValue
                    | Keyword::Result
                    | Keyword::True
                    | Keyword::False
            ),
            // After an error we cannot know, so allow a statement to end
            // there. That limits one lexing mistake to one statement.
            Error => true,
            _ => false,
        }
    }

    /// Does a line starting with this token continue the previous line? The
    /// second half of the newline rule: a newline before `.method` or `+ x`
    /// is not a statement end.
    ///
    /// The spec lists binary operators, `.`, `|>`, and `?`. `|` and `->` are
    /// added (proposed DECISIONS D-21) because the spec's own examples need
    /// them: multi-line variant declarations start lines with `|`, and long
    /// signatures put `-> ReturnType` on its own line.
    pub fn continues_line(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Dot | Question
                | Pipe
                | Plus
                | Minus
                | Star
                | Slash
                | Percent
                | EqEq
                | BangEq
                | Lt
                | LtEq
                | Gt
                | GtEq
                | AndAnd
                | OrOr
                | DotDot
                | DotDotEq
                | FatArrow
                | Bar
                | Arrow
        )
    }

    /// A short description for error messages: "`fn`", "identifier",
    /// "integer literal". Used as "expected `)`, found {describe}".
    pub fn describe(self) -> &'static str {
        use TokenKind::*;
        match self {
            LowerIdent => "identifier",
            UpperIdent => "type or constructor name",
            Kw(kw) => kw.describe(),
            Int { suffix: None } => "integer literal",
            Int { suffix: Some(_) } => "sized integer literal",
            Decimal => "decimal literal",
            Float => "float literal",
            Str { .. } | RawStr => "string literal",
            StrStart { .. } | StrMiddle { .. } | StrEnd { .. } => "string",
            InterpStart => "`#{`",
            InterpEnd => "`}` closing an interpolation",
            DocComment => "doc comment",
            Plus => "`+`",
            Minus => "`-`",
            Star => "`*`",
            Slash => "`/`",
            Percent => "`%`",
            EqEq => "`==`",
            BangEq => "`!=`",
            Lt => "`<`",
            LtEq => "`<=`",
            Gt => "`>`",
            GtEq => "`>=`",
            AndAnd => "`&&`",
            OrOr => "`||`",
            Bang => "`!`",
            Eq => "`=`",
            PlusEq => "`+=`",
            MinusEq => "`-=`",
            StarEq => "`*=`",
            SlashEq => "`/=`",
            Arrow => "`->`",
            FatArrow => "`=>`",
            Pipe => "`|>`",
            Question => "`?`",
            Dot => "`.`",
            DotDot => "`..`",
            DotDotEq => "`..=`",
            Colon => "`:`",
            Comma => "`,`",
            Semicolon => "`;`",
            Bar => "`|`",
            LParen => "`(`",
            RParen => "`)`",
            LBracket => "`[`",
            RBracket => "`]`",
            LBrace => "`{`",
            RBrace => "`}`",
            HashBracket => "`#[`",
            Newline => "newline",
            Eof => "end of file",
            Error => "invalid token",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.describe())
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from the keyword block in `spec/01-lexical.md`. If the spec
    /// changes, this test fails until the enum is updated to match.
    const SPEC_KEYWORDS: &str = "
        as  assert  block  break  cap  const  continue  else  ensures  false  fn  for  handle
        if  impl  import  in  invariant  let  match  needs  opaque  pub  requires
        result  return  scope  self  spawn  test  trait  true  type  var  where  while  with";

    #[test]
    fn keywords_match_the_spec_exactly() {
        let spec: Vec<&str> = SPEC_KEYWORDS.split_whitespace().collect();
        let ours: Vec<&str> = Keyword::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(ours, spec);
    }

    #[test]
    fn keyword_lookup_round_trips() {
        for &kw in Keyword::ALL {
            assert_eq!(Keyword::lookup(kw.as_str()), Some(kw));
            assert_eq!(kw.describe(), format!("`{}`", kw.as_str()));
        }
        assert_eq!(Keyword::lookup("total"), None);
        assert_eq!(Keyword::lookup("Fn"), None, "keywords are case-sensitive");
    }

    #[test]
    fn reserved_words_are_not_keywords() {
        for word in RESERVED_WORDS {
            assert!(is_reserved(word));
            assert_eq!(
                Keyword::lookup(word),
                None,
                "{word} is reserved, not a keyword"
            );
        }
        assert!(!is_reserved("total"));
    }

    #[test]
    fn int_suffixes_parse_and_reject() {
        for (text, ty) in [
            ("i8", "I8"),
            ("i16", "I16"),
            ("i32", "I32"),
            ("i64", "I64"),
            ("u8", "U8"),
            ("u16", "U16"),
            ("u32", "U32"),
            ("u64", "U64"),
        ] {
            assert_eq!(IntSuffix::parse(text).map(IntSuffix::type_name), Some(ty));
        }
        for bad in ["i7", "i128", "f", "f64", "I64", ""] {
            assert_eq!(IntSuffix::parse(bad), None, "{bad:?} is not a suffix");
        }
    }

    #[test]
    fn statement_enders() {
        use TokenKind::*;
        for kind in [
            LowerIdent,
            UpperIdent,
            Int { suffix: None },
            Decimal,
            Str {
                quote: Quote::Single,
            },
            StrEnd {
                quote: Quote::Triple,
            },
            RParen,
            RBracket,
            RBrace,
            Question,
            Kw(Keyword::Return),
            Kw(Keyword::True),
            Kw(Keyword::SelfValue),
        ] {
            assert!(kind.can_end_statement(), "{kind:?} should end a statement");
        }
        for kind in [
            Plus,
            LParen,
            LBrace,
            Comma,
            Eq,
            Arrow,
            StrStart {
                quote: Quote::Single,
            },
            Kw(Keyword::Fn),
            Kw(Keyword::Let),
            Kw(Keyword::Needs),
        ] {
            assert!(
                !kind.can_end_statement(),
                "{kind:?} should not end a statement"
            );
        }
    }

    #[test]
    fn line_continuers() {
        use TokenKind::*;
        // From the spec: binary operators, `.`, `|>`, `?`. Plus `|` and `->` (D-21).
        for kind in [
            Dot, Pipe, Question, Plus, Minus, AndAnd, OrOr, EqEq, FatArrow, Bar, Arrow,
        ] {
            assert!(kind.continues_line(), "{kind:?} should continue the line");
        }
        // Tokens that start a new statement must not glue onto the previous line.
        for kind in [
            LowerIdent,
            Kw(Keyword::Let),
            Kw(Keyword::Return),
            LParen,
            LBracket,
            Bang,
            Eq,
        ] {
            assert!(
                !kind.continues_line(),
                "{kind:?} should not continue the line"
            );
        }
    }

    #[test]
    fn tokens_stay_small() {
        // Span is 12 bytes; the kind should add little. Every token in every
        // file is one of these, so size matters.
        assert!(
            std::mem::size_of::<Token>() <= 16,
            "Token is {} bytes",
            std::mem::size_of::<Token>()
        );
    }
}
