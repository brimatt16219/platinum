//! # Lexing string literals
//!
//! Platinum has three string forms (`spec/01-lexical.md`):
//!
//! | Form          | Example                 | Escapes | `#{}` | Spans lines |
//! |---------------|-------------------------|---------|-------|-------------|
//! | ordinary      | `"total: #{n}\n"`       | yes     | yes   | no          |
//! | triple-quoted | `"""` ... `"""`         | yes     | yes   | yes         |
//! | raw           | `r"C:\x"`, `r#"a"b"#`   | no      | no    | no          |
//!
//! ## Pieces and interpolation
//!
//! A string with no `#{` is one token (`Str` or `RawStr`). A string with
//! interpolations is split into pieces (DECISIONS D-18):
//!
//! ```text
//! "a #{x} b #{y} c"
//! StrStart `"a `   InterpStart `#{`   x   InterpEnd `}`
//! StrMiddle ` b `  InterpStart `#{`   y   InterpEnd `}`   StrEnd ` c"`
//! ```
//!
//! `lex_string_body` scans one piece. When it reaches `#{` it emits the
//! piece, emits `InterpStart`, pushes an `Interp` entry on the mode stack, and
//! *returns*: the main loop then lexes the embedded expression as normal
//! code. When the main loop sees the `}` that closes the interpolation
//! (`lex_inside_interp`), it pops the entry and calls `lex_string_body` again
//! to continue the same string.
//!
//! ## What the lexer checks and what it leaves alone
//!
//! The lexer *validates* escapes (`\q` is `PT0004`) but does not *decode*
//! them: token spans still cover the raw source text, and the parser turns
//! `\n` into a newline character. That keeps tokens small and lossless. It
//! also does not strip the indentation of triple-quoted strings; the parser
//! does that, which is why string tokens carry their `Quote`.
//!
//! ## Recovery
//!
//! Every way a string can go wrong still produces a balanced token stream
//! (each `InterpStart` gets an `InterpEnd`, each `StrStart` an `StrEnd`),
//! using zero-width tokens where the source is missing something. The parser
//! can then carry on as if the string were well formed, and the user sees
//! one diagnostic instead of a cascade.

use plat_diag::{Diagnostic, Span, codes};

use super::{Interp, Lexer};
use crate::token::{Quote, TokenKind};

impl Lexer<'_> {
    // --- Ordinary and triple-quoted strings ----------------------------

    /// Lexes a string starting at the opening `"` or `"""`.
    pub(super) fn lex_string(&mut self, start: usize) {
        let quote = if self.src[self.pos..].starts_with("\"\"\"") {
            self.pos += 3;
            Quote::Triple
        } else {
            self.pos += 1;
            Quote::Single
        };
        self.lex_string_body(start, quote, start, true);
    }

    /// Scans one piece of a string, starting just after an opening quote
    /// (`first == true`) or just after the `}` that closed an interpolation.
    ///
    /// - `piece_start`: where this piece's token begins.
    /// - `string_start`: where the whole string's opening quote is, so errors
    ///   in later pieces still point at the start of the string.
    fn lex_string_body(
        &mut self,
        piece_start: usize,
        quote: Quote,
        string_start: usize,
        first: bool,
    ) {
        let whole = if first {
            TokenKind::Str { quote }
        } else {
            TokenKind::StrEnd { quote }
        };
        loop {
            match self.peek() {
                None => {
                    self.unterminated_string(string_start, quote);
                    self.push(whole, piece_start);
                    return;
                }
                // Ordinary strings stop at the end of the line. The newline
                // itself is not part of the string.
                Some('\n') if quote == Quote::Single => {
                    self.unterminated_string(string_start, quote);
                    self.push(whole, piece_start);
                    return;
                }
                Some('\\') => self.lex_escape(),
                Some('#') if self.peek_second() == Some('{') => {
                    let piece = if first {
                        TokenKind::StrStart { quote }
                    } else {
                        TokenKind::StrMiddle { quote }
                    };
                    self.push(piece, piece_start);
                    let interp_start = self.pos;
                    self.pos += 2;
                    self.push(TokenKind::InterpStart, interp_start);
                    self.interps.push(Interp {
                        quote,
                        depth: 0,
                        string_start,
                        interp_start,
                    });
                    return;
                }
                Some('"') => match quote {
                    Quote::Single => {
                        self.pos += 1;
                        self.push(whole, piece_start);
                        return;
                    }
                    // Inside `"""`, lone `"` and `""` are ordinary text. The
                    // first `"""` closes the string.
                    Quote::Triple if self.src[self.pos..].starts_with("\"\"\"") => {
                        self.pos += 3;
                        self.push(whole, piece_start);
                        return;
                    }
                    Quote::Triple => self.pos += 1,
                },
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    fn unterminated_string(&mut self, string_start: usize, quote: Quote) {
        let opener_len = match quote {
            Quote::Single => 1,
            Quote::Triple => 3,
        };
        let opener = Span::new(
            self.file,
            string_start as u32,
            (string_start + opener_len) as u32,
        );
        let mut d = Diagnostic::error(
            codes::UNTERMINATED_STRING,
            "unterminated string literal",
            opener,
        )
        .with_primary_label("this string is never closed");
        d = match quote {
            Quote::Single => d.with_note(
                "strings must end on the line they start on; use \"\"\" ... \"\"\" for text that spans lines",
            ),
            Quote::Triple => d.with_note("a triple-quoted string ends at the next `\"\"\"`"),
        };
        self.report(d);
    }

    // --- Escapes -------------------------------------------------------

    /// Validates one escape sequence starting at `\`. Valid escapes:
    /// `\n \t \r \\ \" \#` and `\u{...}` with 1 to 6 hex digits naming a
    /// Unicode scalar value.
    fn lex_escape(&mut self) {
        let start = self.pos;
        self.pos += 1; // the backslash
        match self.peek() {
            Some('n' | 't' | 'r' | '\\' | '"' | '#') => {
                self.pos += 1;
            }
            Some('u') => {
                self.pos += 1;
                self.lex_unicode_escape(start);
            }
            // A backslash right before the end of the line: leave the newline
            // for the string loop, which reports the unterminated string.
            None | Some('\n') => {
                self.report(
                    Diagnostic::error(
                        codes::INVALID_ESCAPE,
                        "backslash at end of line",
                        self.span_from(start),
                    )
                    .with_primary_label("nothing to escape here")
                    .with_note("to continue text on the next line, use a triple-quoted string"),
                );
            }
            Some(other) => {
                self.bump();
                let d = Diagnostic::error(
                    codes::INVALID_ESCAPE,
                    format!("invalid escape sequence `\\{other}`"),
                    self.span_from(start),
                )
                .with_primary_label("not a valid escape")
                .with_note("valid escapes: \\n \\t \\r \\\\ \\\" \\# \\u{1F600}");
                self.report(d);
            }
        }
    }

    /// The `{XXXX}` part of `\u{XXXX}`; `start` is the backslash.
    fn lex_unicode_escape(&mut self, start: usize) {
        if !self.eat('{') {
            let d = Diagnostic::error(
                codes::INVALID_ESCAPE,
                "expected `{` after `\\u`",
                self.span_from(start),
            )
            .with_primary_label("write Unicode escapes as `\\u{1F600}`");
            self.report(d);
            return;
        }
        let digits_start = self.pos;
        self.eat_while(|c| c.is_ascii_hexdigit());
        let digits = &self.src[digits_start..self.pos];

        if !self.eat('}') {
            let d = Diagnostic::error(
                codes::INVALID_ESCAPE,
                "unterminated `\\u{` escape",
                self.span_from(start),
            )
            .with_primary_label("expected hex digits followed by `}`");
            self.report(d);
            return;
        }

        let problem = if digits.is_empty() || digits.len() > 6 {
            Some("a `\\u{...}` escape needs 1 to 6 hex digits")
        } else {
            // At most 6 hex digits, so this fits in a u32 and parsing succeeds.
            match u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
            {
                Some(_) => None,
                None => Some(
                    "not a valid Unicode scalar value (surrogates and values above 10FFFF are not allowed)",
                ),
            }
        };
        if let Some(message) = problem {
            let d = Diagnostic::error(codes::INVALID_ESCAPE, message, self.span_from(start))
                .with_primary_label("invalid Unicode escape");
            self.report(d);
        }
    }

    // --- Raw strings ---------------------------------------------------

    /// Is the cursor at `r"` or `r#...#"`? (Otherwise `r` starts a name.)
    pub(super) fn raw_string_ahead(&self) -> bool {
        let rest = &self.src[self.pos..];
        let Some(after_r) = rest.strip_prefix('r') else {
            return false;
        };
        after_r.trim_start_matches('#').starts_with('"')
    }

    /// Lexes `r"..."` or `r#"..."#` (any number of `#`). Raw strings have no
    /// escapes and no interpolation; they end at a `"` followed by as many
    /// `#` as the opening had, which is how `r#"say "hi""#` can contain `"`.
    pub(super) fn lex_raw_string(&mut self, start: usize) {
        self.pos += 1; // `r`
        let hashes_start = self.pos;
        self.eat_while(|c| c == '#');
        let hashes = self.pos - hashes_start;
        self.pos += 1; // the opening `"` (checked by raw_string_ahead)

        loop {
            match self.peek() {
                None | Some('\n') => {
                    let opener = Span::new(self.file, start as u32, (start + 2 + hashes) as u32);
                    let closer = format!("\"{}", "#".repeat(hashes));
                    let d = Diagnostic::error(
                        codes::UNTERMINATED_STRING,
                        "unterminated raw string literal",
                        opener,
                    )
                    .with_primary_label("this raw string is never closed")
                    .with_note(format!(
                        "this raw string must end with `{closer}` on the same line"
                    ));
                    self.report(d);
                    self.push(TokenKind::RawStr, start);
                    return;
                }
                Some('"') => {
                    let after_quote = &self.src[self.pos + 1..];
                    let closes = after_quote.len() >= hashes
                        && after_quote.bytes().take(hashes).all(|b| b == b'#');
                    self.pos += 1;
                    if closes {
                        self.pos += hashes;
                        self.push(TokenKind::RawStr, start);
                        return;
                    }
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    // --- Inside an interpolation ----------------------------------------

    /// Handles the characters whose meaning depends on the mode stack while
    /// inside `#{ ... }`. Returns `true` if it consumed input, `false` to let
    /// the main loop lex the character as ordinary code.
    pub(super) fn lex_inside_interp(&mut self, c: char, start: usize) -> bool {
        // Copy the top entry (it is small and `Copy`) so we can read it while
        // calling other `&mut self` methods. We only reach back into the
        // stack with `last_mut` for the two lines that change `depth`.
        let Some(&top) = self.interps.last() else {
            return false;
        };
        match c {
            '{' => {
                if let Some(t) = self.interps.last_mut() {
                    t.depth += 1;
                }
                self.pos += 1;
                self.push(TokenKind::LBrace, start);
                true
            }
            '}' if top.depth > 0 => {
                if let Some(t) = self.interps.last_mut() {
                    t.depth -= 1;
                }
                self.pos += 1;
                self.push(TokenKind::RBrace, start);
                true
            }
            '}' => {
                self.interps.pop();
                self.pos += 1;
                self.push(TokenKind::InterpEnd, start);
                self.lex_string_body(self.pos, top.quote, top.string_start, false);
                true
            }
            // An interpolation in a one-line string must close on that line.
            '\n' if top.quote == Quote::Single => {
                self.interps.pop();
                self.unterminated_interp(top, "the line ends before it is closed", true);
                true
            }
            // `puts("total: #{n")`: the `}` was forgotten and this `"` was
            // meant to end the string. If no `}` follows on this line, the
            // quote cannot be the start of a nested string that the
            // interpolation then closes, so treat it as the string's end.
            '"' if top.quote == Quote::Single && !self.close_brace_later_on_line() => {
                self.interps.pop();
                self.unterminated_interp(top, "the string ends before it is closed", false);
                self.pos += 1;
                self.push(
                    TokenKind::StrEnd {
                        quote: Quote::Single,
                    },
                    start,
                );
                true
            }
            _ => false,
        }
    }

    fn close_brace_later_on_line(&self) -> bool {
        let rest = &self.src[self.pos + 1..];
        let line = rest.split('\n').next().unwrap_or("");
        line.contains('}')
    }

    /// Reports an interpolation that never got its `}` and emits a zero-width
    /// `InterpEnd` so the stream stays balanced. With `close_string`, also
    /// emits a zero-width `StrEnd`; the stray-quote case passes `false`
    /// because it emits a real `StrEnd` for the quote itself.
    pub(super) fn unterminated_interp(&mut self, interp: Interp, why: &str, close_string: bool) {
        let opener = Span::new(
            self.file,
            interp.interp_start as u32,
            interp.interp_start as u32 + 2,
        );
        let d = Diagnostic::error(
            codes::UNTERMINATED_INTERPOLATION,
            "unterminated string interpolation",
            opener,
        )
        .with_primary_label(format!("this `#{{` is never closed; {why}"))
        .with_note(
            "close the interpolation with `}`; to write a literal `#{`, escape it as `\\#{`",
        );
        self.report(d);

        let here = self.pos;
        self.push(TokenKind::InterpEnd, here);
        if close_string {
            self.push(
                TokenKind::StrEnd {
                    quote: interp.quote,
                },
                here,
            );
        }
    }
}
