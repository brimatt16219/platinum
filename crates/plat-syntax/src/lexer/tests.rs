//! Lexer tests.
//!
//! Most tests render the token stream as text with `dump` and compare it to
//! an `insta` snapshot in `src/lexer/snapshots/`. Each line shows the token
//! kind, its byte span, and the exact source text it covers, so a snapshot
//! reads like a table you can check by eye. Diagnostics follow the tokens.

use plat_diag::{FileId, SourceFile};

use super::{Lexed, lex};
use crate::token::TokenKind;

fn lex_str(src: &str) -> Lexed {
    let file = SourceFile::new("test.plat", src).expect("small test input");
    lex(FileId(0), &file)
}

/// One line per token (`kind  start..end  "text"`), then one line per
/// diagnostic (`PT0005 start..end message`).
fn dump(src: &str) -> String {
    let lexed = lex_str(src);
    let mut out = String::new();
    for t in &lexed.tokens {
        let text = &src[t.span.start as usize..t.span.end as usize];
        let kind = format!("{:?}", t.kind);
        out.push_str(&format!(
            "{kind:<32} {:>3}..{:<3} {text:?}\n",
            t.span.start, t.span.end
        ));
    }
    for d in &lexed.diagnostics {
        out.push_str(&format!(
            "{} {}..{} {}\n",
            d.code, d.primary.span.start, d.primary.span.end, d.message
        ));
    }
    out
}

fn kinds(src: &str) -> Vec<TokenKind> {
    lex_str(src).tokens.into_iter().map(|t| t.kind).collect()
}

// --- Snapshots, one per token family ------------------------------------

#[test]
fn identifiers_and_keywords() {
    insta::assert_snapshot!(dump(
        "fn refund let var total _ _tmp x1 Money Some HTTP2 self result true false pubx"
    ));
}

#[test]
fn every_keyword() {
    let all: Vec<&str> = crate::token::Keyword::ALL
        .iter()
        .map(|k| k.as_str())
        .collect();
    insta::assert_snapshot!(dump(&all.join(" ")));
}

#[test]
fn integers() {
    insta::assert_snapshot!(dump("0 42 1_000_000 0xFF 0xdead_beef 0b1010 007"));
}

#[test]
fn sized_integers() {
    insta::assert_snapshot!(dump(
        "42i8 7i16 7i32 42i64 255u8 1u16 1u32 1u64 0xFFu8 0b1i8"
    ));
}

#[test]
fn decimals_and_floats() {
    insta::assert_snapshot!(dump(
        "1.50 0.1 1_000.00 1.5f 2.0e-3f 6.02e23f 1e5f 1f 3E+2f"
    ));
}

#[test]
fn numbers_next_to_dots() {
    // range, method call, tuple index chain
    insta::assert_snapshot!(dump("1..5 1..=5 1.max(2) pair.0.1 x.0"));
}

#[test]
fn malformed_numbers() {
    insta::assert_snapshot!(dump(
        "0x 0b 0b102 1_ 1__0 0x_FF 1.5_ 1e5 42i7 1.5i32 7abc 0b1f"
    ));
}

#[test]
fn operators() {
    insta::assert_snapshot!(dump(
        "+ - * / % == != < <= > >= && || ! = += -= *= /= -> => |> ? . .. ..= : , ; |"
    ));
}

#[test]
fn delimiters_and_attributes() {
    insta::assert_snapshot!(dump("( ) [ ] { } #[derive(Eq)]"));
}

#[test]
fn maximal_munch_without_spaces() {
    insta::assert_snapshot!(dump("a!=b x->y a..=b a...b x=>y a|>f(b)||c a&&b"));
}

#[test]
fn comments() {
    insta::assert_snapshot!(dump(
        "a // line comment\n/// doc comment\n//// not doc\nb /* block /* nested */ still */ c"
    ));
}

#[test]
fn unterminated_block_comments() {
    insta::assert_snapshot!(dump("a /* never closed"));
    insta::assert_snapshot!("unterminated_nested", dump("a /* outer /* inner */"));
}

#[test]
fn unexpected_characters() {
    insta::assert_snapshot!(dump(
        "let price = $5\nx @@ y\n&\n#\nlet s = \u{201C}hi\u{201D}\ncafé"
    ));
}

#[test]
fn reserved_words() {
    insta::assert_snapshot!(dump("let yield = async + decreases"));
}

// --- Targeted behavior ---------------------------------------------------

#[test]
fn byte_order_mark_is_skipped() {
    let lexed = lex_str("\u{FEFF}fn");
    assert_eq!(
        lexed.tokens[0].kind,
        TokenKind::Kw(crate::token::Keyword::Fn)
    );
    assert_eq!(lexed.tokens[0].span.start, 3);
    assert!(lexed.diagnostics.is_empty());
}

#[test]
fn empty_input_is_just_eof() {
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
    assert_eq!(kinds("   \n\t // only a comment"), vec![TokenKind::Eof]);
}

#[test]
fn valid_examples_produce_no_errors_before_strings_exist() {
    // Until step 9 adds strings, check a string-free slice of real syntax.
    let src = "fn area(s: Shape) -> Float {\n  match s {\n    Circle(r) => 3.14159f * r * r,\n    Rect(w, h) => w * h,\n  }\n}";
    let lexed = lex_str(src);
    assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
    assert!(!lexed.tokens.iter().any(|t| t.kind == TokenKind::Error));
}

/// The lexer must never panic, and its output must always be well formed,
/// whatever the input. This feeds it every pair of ASCII characters (16,384
/// inputs) plus some multibyte and adversarial strings, and checks the
/// invariants every later phase relies on.
#[test]
fn never_panics_and_spans_are_well_formed() {
    let mut inputs: Vec<String> = Vec::new();
    for a in 0u8..128 {
        for b in 0u8..128 {
            inputs.push(format!("{}{}", a as char, b as char));
        }
    }
    for s in [
        "/*",
        "/* /*",
        "*/",
        "0x",
        "0b",
        "1e",
        "1e+",
        "1.",
        "1.e5",
        "..",
        "...",
        "#",
        "\u{FEFF}",
        "\u{FEFF}\u{FEFF}",
        "€",
        "a€b",
        "😀😀",
        "1€",
        "0x€",
        "/// €",
        "_",
        "1_€",
        "é",
        "\u{201C}",
        "\0",
        "\r\n\r\n",
    ] {
        inputs.push(s.to_string());
    }

    for src in &inputs {
        let lexed = lex_str(src);
        let tokens = &lexed.tokens;

        let last = tokens.last().expect("at least Eof");
        assert_eq!(last.kind, TokenKind::Eof, "input {src:?}");
        assert_eq!(last.span.start as usize, src.len(), "input {src:?}");

        let mut prev_end = 0;
        for t in tokens {
            let (s, e) = (t.span.start as usize, t.span.end as usize);
            assert!(
                s <= e && e <= src.len(),
                "span out of range for {src:?}: {t:?}"
            );
            assert!(s >= prev_end, "overlapping tokens for {src:?}: {t:?}");
            assert!(
                src.get(s..e).is_some(),
                "span not on char boundaries for {src:?}: {t:?}"
            );
            prev_end = e;
        }
        for d in &lexed.diagnostics {
            let span = d.primary.span;
            assert!(
                src.get(span.start as usize..span.end as usize).is_some(),
                "bad diagnostic span for {src:?}"
            );
        }
    }
}
