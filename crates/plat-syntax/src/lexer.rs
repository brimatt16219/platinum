//! # The lexer: characters in, tokens out
//!
//! A lexer (also called a scanner or tokenizer) is the first phase of a
//! compiler. It walks the source text one character at a time and groups
//! characters into tokens: `fn`, `refund`, `(`, `1_000`, and so on. Later
//! phases never look at raw characters again.
//!
//! ## How this one works
//!
//! The lexer is hand-written (CLAUDE.md prefers that over a generator, both
//! for learning and for Platinum's unusual `?`/`!` rule). The core is a
//! **cursor**: a byte position `pos` into the source plus three tiny helpers.
//!
//! - `peek()` looks at the next character without consuming it.
//! - `bump()` consumes one character and advances `pos` by its UTF-8 length.
//! - `eat_while(pred)` consumes characters as long as `pred` says yes.
//!
//! The main loop (`run`) peeks at one character and dispatches on it: a letter
//! starts an identifier, a digit starts a number, `/` might start a comment or
//! be the division operator, and so on. Each `lex_*` method consumes exactly
//! one token's worth of characters and pushes a `Token` whose span runs from
//! where it started to where the cursor ended.
//!
//! ## Maximal munch
//!
//! When characters could form a short or a long token, the lexer takes the
//! longest: `..=` is one token, not `..` then `=`; `->` is one token, not `-`
//! then `>`. This is called **maximal munch** and nearly every lexer uses it.
//!
//! ## Errors never stop the lexer
//!
//! Bad input becomes a diagnostic plus a best-guess token, and lexing
//! continues. A malformed number like `0b102` still produces an `Int` token
//! (so the parser does not trip over a hole), and a run of junk characters
//! like `$$$` becomes one `Error` token with one diagnostic, not three.
//! The lexer must never panic on any input; a test feeds it every pair of
//! ASCII characters to check.
//!
//! ## What is not here yet
//!
//! As of M0 step 8, still to come:
//! - step 9: string literals and interpolation (until then `"` is reported
//!   as an unexpected character),
//! - step 10: the newline rule (until then newlines are plain whitespace).

use plat_diag::{Diagnostic, DiagnosticBag, FileId, SourceFile, Span, codes};

use crate::token::{IntSuffix, Keyword, Token, TokenKind, is_reserved};

/// The lexer's output: every token (ending with `Eof`) and every problem found.
#[derive(Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: DiagnosticBag,
}

/// Lexes a whole file.
///
/// Takes a `SourceFile` rather than a bare `&str` because `SourceFile::new`
/// has already checked that the text fits in `u32` offsets, so every span the
/// lexer creates is valid.
pub fn lex(file: FileId, source: &SourceFile) -> Lexed {
    let mut lexer = Lexer {
        file,
        src: source.text(),
        pos: 0,
        tokens: Vec::new(),
        diagnostics: DiagnosticBag::new(),
    };
    lexer.run();
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'src> {
    file: FileId,
    src: &'src str,
    /// Byte offset of the next unread character. Always on a char boundary.
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: DiagnosticBag,
}

impl<'src> Lexer<'src> {
    // --- The cursor ----------------------------------------------------

    /// The next character, without consuming it.
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// The character after the next one.
    fn peek_second(&self) -> Option<char> {
        self.src[self.pos..].chars().nth(1)
    }

    /// The character two after the next one.
    fn peek_third(&self) -> Option<char> {
        self.src[self.pos..].chars().nth(2)
    }

    /// Consumes and returns the next character.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Consumes the next character if it is `expected`.
    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    /// Consumes characters while `pred` holds.
    fn eat_while(&mut self, pred: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if !pred(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
    }

    // --- Building output -----------------------------------------------

    /// Span from `start` to the current position. The casts are safe because
    /// `SourceFile::new` guarantees the text length fits in `u32`.
    fn span_from(&self, start: usize) -> Span {
        Span::new(self.file, start as u32, self.pos as u32)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let span = self.span_from(start);
        self.tokens.push(Token::new(kind, span));
    }

    fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn previous_kind(&self) -> Option<TokenKind> {
        self.tokens.last().map(|t| t.kind)
    }

    // --- The main loop -------------------------------------------------

    fn run(&mut self) {
        // A UTF-8 byte order mark is invisible editor metadata, not code.
        if self.src.starts_with('\u{FEFF}') {
            self.pos = '\u{FEFF}'.len_utf8();
        }

        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                // Newlines become significant in step 10 (DECISIONS D-17).
                ' ' | '\t' | '\r' | '\n' => {
                    self.bump();
                }
                '/' if self.peek_second() == Some('/') => self.lex_line_comment(start),
                '/' if self.peek_second() == Some('*') => self.lex_block_comment(start),
                'a'..='z' | '_' => self.lex_lower_ident(start),
                'A'..='Z' => self.lex_upper_ident(start),
                '0'..='9' => self.lex_number(start),
                _ => self.lex_operator_or_junk(start),
            }
        }

        let end = self.pos;
        self.push(TokenKind::Eof, end);
    }

    // --- Comments ------------------------------------------------------

    /// `// ...` is skipped. `/// ...` (exactly three slashes) becomes a
    /// `DocComment` token; `////` is an ordinary comment, as in Rust.
    fn lex_line_comment(&mut self, start: usize) {
        self.eat_while(|c| c != '\n');
        let text = &self.src[start..self.pos];
        if text.starts_with("///") && !text.starts_with("////") {
            self.push(TokenKind::DocComment, start);
        }
    }

    /// `/* ... */` comments nest: `/* a /* b */ c */` is one comment. We count
    /// depth instead of stopping at the first `*/`.
    fn lex_block_comment(&mut self, start: usize) {
        self.pos += 2; // the opening `/*`
        let mut depth = 1;
        while depth > 0 {
            match (self.peek(), self.peek_second()) {
                (Some('/'), Some('*')) => {
                    self.pos += 2;
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.pos += 2;
                    depth -= 1;
                }
                (Some(_), _) => {
                    self.bump();
                }
                (None, _) => {
                    let opener = Span::new(self.file, start as u32, start as u32 + 2);
                    let mut d = Diagnostic::error(
                        codes::UNTERMINATED_BLOCK_COMMENT,
                        "unterminated block comment",
                        opener,
                    )
                    .with_primary_label("this comment is never closed");
                    if depth > 1 {
                        d = d.with_note(format!(
                            "block comments nest; {depth} `/*` are still open at the end of the file"
                        ));
                    }
                    self.report(d);
                    return;
                }
            }
        }
    }

    // --- Identifiers and keywords ----------------------------------------

    fn is_ident_continue(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Lexes a lowercase identifier or keyword, including the Ruby-style
    /// suffix rule from `spec/01-lexical.md` (DECISIONS D-16):
    ///
    /// - `?` directly after the name (no space) becomes part of it:
    ///   `empty?`, `paid?`. By convention these return `Bool`.
    /// - `!` directly after the name becomes part of it, *unless* the next
    ///   character is `=`: `sort!` is one name, but `a!=b` is `a != b`.
    /// - At most one suffix: `x??` is the name `x?` then the operator `?`.
    /// - Keywords never take a suffix: `self?` is `self` followed by the `?`
    ///   operator, since a keyword cannot be a name.
    ///
    /// The lexer does not decide what `name?` *means*. If `name?` turns out
    /// not to exist but `name` is a `Result` or `Option`, the name resolver
    /// (M2) reads it as `(name)?`, error propagation. Keeping that decision
    /// out of the lexer means the lexer never needs to know what is in scope.
    fn lex_lower_ident(&mut self, start: usize) {
        self.eat_while(Self::is_ident_continue);
        let base = &self.src[start..self.pos];

        if let Some(kw) = Keyword::lookup(base) {
            self.push(TokenKind::Kw(kw), start);
            return;
        }

        match (self.peek(), self.peek_second()) {
            (Some('?'), _) => self.pos += 1,
            (Some('!'), next) if next != Some('=') => self.pos += 1,
            _ => {}
        }

        // Check the base word, so `yield?` is caught too: once `yield` is a
        // keyword, `yield?` would lex as `yield` then `?`.
        if is_reserved(base) {
            // Report, but still produce an identifier so parsing can go on.
            let span = self.span_from(start);
            self.report(
                Diagnostic::error(
                    codes::RESERVED_KEYWORD,
                    format!("`{base}` is reserved for future use"),
                    span,
                )
                .with_primary_label("reserved word used as a name"),
            );
        }
        self.push(TokenKind::LowerIdent, start);
    }

    /// Uppercase names (types, constructors) never take a `?`/`!` suffix:
    /// `Some?` is `Some` followed by the `?` operator.
    fn lex_upper_ident(&mut self, start: usize) {
        self.eat_while(Self::is_ident_continue);
        self.push(TokenKind::UpperIdent, start);
    }

    // --- Numbers -------------------------------------------------------

    /// Lexes every number form from `spec/01-lexical.md` (details in
    /// DECISIONS D-22):
    ///
    /// ```text
    /// 42  1_000  0xFF  0b1010      Int
    /// 42i64  255u8  0xFFu8         Int with a size suffix
    /// 1.50                         Decimal (exact)
    /// 1.5f  2.0e-3f  1f            Float
    /// ```
    ///
    /// At most one diagnostic is reported per literal (the first problem),
    /// and a token of the intended kind is always produced.
    fn lex_number(&mut self, start: usize) {
        let mut problem: Option<Diagnostic> = None;
        let mut note_problem = |d: Diagnostic| {
            problem.get_or_insert(d);
        };

        // Base prefix.
        let radix = match (self.peek(), self.peek_second()) {
            (Some('0'), Some('x')) => 16,
            (Some('0'), Some('b')) => 2,
            _ => 10,
        };
        if radix != 10 {
            self.pos += 2;
        }

        // Integer digits. Binary literals consume all decimal digits so that
        // `0b102` is one malformed literal, not `0b10` followed by `2`.
        let digits_start = self.pos;
        if radix == 16 {
            self.eat_while(|c| c.is_ascii_hexdigit() || c == '_');
        } else {
            self.eat_while(|c| c.is_ascii_digit() || c == '_');
        }
        let digits = &self.src[digits_start..self.pos];

        if digits.is_empty() {
            let prefix = if radix == 16 { "0x" } else { "0b" };
            note_problem(
                Diagnostic::error(
                    codes::MALFORMED_NUMBER,
                    format!("no digits after `{prefix}`"),
                    self.span_from(start),
                )
                .with_primary_label("expected at least one digit here"),
            );
        } else if radix == 2
            // A "let chain" (Rust 2024): the branch runs only if radix is 2
            // AND a non-binary digit is found, binding it as `bad`.
            && let Some(bad) = digits.chars().find(|c| !matches!(c, '0' | '1' | '_'))
        {
            note_problem(
                Diagnostic::error(
                    codes::MALFORMED_NUMBER,
                    format!("`{bad}` is not a binary digit"),
                    self.span_from(start),
                )
                .with_primary_label("binary literals may only contain `0` and `1`"),
            );
        }
        if let Some(d) = self.check_underscores(digits, digits_start) {
            note_problem(d);
        }

        // Fractional part: only `.` followed by a digit. `1.max(2)` and `1..5`
        // keep the `.` for the method call or range. After a `.` token we never
        // read a fraction, so tuple access `pair.0.1` lexes as `pair . 0 . 1`
        // instead of `pair . 0.1`.
        let after_dot = self.previous_kind() == Some(TokenKind::Dot);
        let mut is_fractional = false;
        if radix == 10
            && !after_dot
            && self.peek() == Some('.')
            && self.peek_second().is_some_and(|c| c.is_ascii_digit())
        {
            self.pos += 1;
            let frac_start = self.pos;
            self.eat_while(|c| c.is_ascii_digit() || c == '_');
            let frac = &self.src[frac_start..self.pos];
            if let Some(d) = self.check_underscores(frac, frac_start) {
                note_problem(d);
            }
            is_fractional = true;
        }

        // Exponent: `e3`, `e-3`, `E+3`. Only valid on floats (checked below).
        let mut has_exponent = false;
        if radix == 10 && matches!(self.peek(), Some('e' | 'E')) {
            let digit_follows = match self.peek_second() {
                Some(c) if c.is_ascii_digit() => true,
                Some('+' | '-') => self.peek_third().is_some_and(|c| c.is_ascii_digit()),
                _ => false,
            };
            if digit_follows {
                self.pos += 1;
                if matches!(self.peek(), Some('+' | '-')) {
                    self.pos += 1;
                }
                self.eat_while(|c| c.is_ascii_digit() || c == '_');
                has_exponent = true;
            }
        }

        // Suffix: any identifier characters glued to the number.
        let suffix_start = self.pos;
        if self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            self.eat_while(Self::is_ident_continue);
        }
        let suffix = &self.src[suffix_start..self.pos];
        let suffix_span = self.span_from(suffix_start);

        let kind = match suffix {
            "" if has_exponent => {
                note_problem(
                    Diagnostic::error(
                        codes::MALFORMED_NUMBER,
                        "an exponent is only allowed on a Float literal",
                        self.span_from(start),
                    )
                    .with_primary_label("add an `f` suffix to make this a Float")
                    .with_note(
                        "Decimal literals are exact and written out in full, such as `1000.0`",
                    ),
                );
                TokenKind::Float
            }
            "" if is_fractional => TokenKind::Decimal,
            "" => TokenKind::Int { suffix: None },
            "f" if radix == 10 => TokenKind::Float,
            other => match IntSuffix::parse(other) {
                Some(_) if is_fractional || has_exponent => {
                    note_problem(
                        Diagnostic::error(
                            codes::UNKNOWN_NUMBER_SUFFIX,
                            format!("integer suffix `{other}` on a decimal literal"),
                            suffix_span,
                        )
                        .with_primary_label("sized integer suffixes only apply to whole numbers"),
                    );
                    TokenKind::Decimal
                }
                Some(size) => TokenKind::Int { suffix: Some(size) },
                None => {
                    note_problem(
                        Diagnostic::error(
                            codes::UNKNOWN_NUMBER_SUFFIX,
                            format!("unknown suffix `{other}` on number literal"),
                            suffix_span,
                        )
                        .with_primary_label("not a valid suffix")
                        .with_note(
                            "valid suffixes: i8 i16 i32 i64 u8 u16 u32 u64, and f for Float",
                        ),
                    );
                    if is_fractional || has_exponent {
                        TokenKind::Decimal
                    } else {
                        TokenKind::Int { suffix: None }
                    }
                }
            },
        };

        if let Some(d) = problem {
            self.report(d);
        }
        self.push(kind, start);
    }

    /// Underscores separate digits, so each one must sit between two digits:
    /// `1_000` is fine; `1_`, `0x_FF`, and `1__0` are not (DECISIONS D-22).
    fn check_underscores(&self, digits: &str, digits_start: usize) -> Option<Diagnostic> {
        let bytes = digits.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b != b'_' {
                continue;
            }
            let before_ok = i > 0 && bytes[i - 1] != b'_';
            let after_ok = i + 1 < bytes.len() && bytes[i + 1] != b'_';
            if !(before_ok && after_ok) {
                let at = (digits_start + i) as u32;
                return Some(
                    Diagnostic::error(
                        codes::MALFORMED_NUMBER,
                        "misplaced `_` in number literal",
                        Span::new(self.file, at, at + 1),
                    )
                    .with_primary_label("`_` must sit between two digits"),
                );
            }
        }
        None
    }

    // --- Operators and junk ----------------------------------------------

    /// Lexes an operator or punctuation token using maximal munch. If the
    /// character cannot start any token, reports it (and any directly
    /// following junk) as one unexpected-character error.
    fn lex_operator_or_junk(&mut self, start: usize) {
        use TokenKind::*;

        let Some(c) = self.bump() else { return };
        let kind = match c {
            '+' => {
                if self.eat('=') {
                    PlusEq
                } else {
                    Plus
                }
            }
            '-' => {
                if self.eat('>') {
                    Arrow
                } else if self.eat('=') {
                    MinusEq
                } else {
                    Minus
                }
            }
            '*' => {
                if self.eat('=') {
                    StarEq
                } else {
                    Star
                }
            }
            '/' => {
                if self.eat('=') {
                    SlashEq
                } else {
                    Slash
                }
            }
            '%' => Percent,
            '=' => {
                if self.eat('=') {
                    EqEq
                } else if self.eat('>') {
                    FatArrow
                } else {
                    Eq
                }
            }
            '!' => {
                if self.eat('=') {
                    BangEq
                } else {
                    Bang
                }
            }
            '<' => {
                if self.eat('=') {
                    LtEq
                } else {
                    Lt
                }
            }
            '>' => {
                if self.eat('=') {
                    GtEq
                } else {
                    Gt
                }
            }
            '&' if self.eat('&') => AndAnd,
            // `||` is also how a zero-parameter lambda starts (`|| 42`); the
            // parser accepts `OrOr` there.
            '|' => {
                if self.eat('|') {
                    OrOr
                } else if self.eat('>') {
                    Pipe
                } else {
                    Bar
                }
            }
            '.' => {
                if self.eat('.') {
                    if self.eat('=') { DotDotEq } else { DotDot }
                } else {
                    Dot
                }
            }
            '?' => Question,
            ':' => Colon,
            ',' => Comma,
            ';' => Semicolon,
            '(' => LParen,
            ')' => RParen,
            '[' => LBracket,
            ']' => RBracket,
            '{' => LBrace,
            '}' => RBrace,
            '#' if self.eat('[') => HashBracket,
            _ => {
                self.lex_junk(start);
                return;
            }
        };
        self.push(kind, start);
    }

    /// Could `c` begin a valid token (or whitespace)? Used to decide where a
    /// run of junk characters ends.
    fn can_start_token(c: char) -> bool {
        c.is_ascii_alphanumeric()
            || c.is_ascii_whitespace()
            || "_+-*/%=!<>&|.?:,;()[]{}#\"".contains(c)
    }

    /// Reports a run of characters that cannot start any token, such as `$`,
    /// `@@`, or a curly quote, as a single error. The first character has
    /// already been consumed.
    fn lex_junk(&mut self, start: usize) {
        self.eat_while(|c| !Self::can_start_token(c));
        let text = &self.src[start..self.pos];
        let span = self.span_from(start);

        let plural = if text.chars().count() == 1 {
            "character"
        } else {
            "characters"
        };
        let mut d = Diagnostic::error(
            codes::UNEXPECTED_CHARACTER,
            format!("unexpected {plural} `{text}`"),
            span,
        )
        .with_primary_label("not valid Platinum syntax here");
        if text.contains(['\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}']) {
            d = d.with_note("this looks like a curly quote; Platinum strings use plain `\"`");
        } else if text.chars().any(char::is_alphabetic) {
            d = d.with_note(
                "names may only use ASCII letters, digits, and `_`; other characters are allowed in strings and comments",
            );
        }
        self.report(d);
        self.push(TokenKind::Error, start);
    }
}

#[cfg(test)]
mod tests;
