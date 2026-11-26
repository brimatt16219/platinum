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

#[test]
fn question_and_bang_suffixes() {
    // Absorbed: empty? paid? sort! push! a! (next char is a space, not `=`)
    // Not absorbed: after `)` `]` a literal or whitespace; after keywords and
    // uppercase names; `!` before `=`; a second suffix.
    insta::assert_snapshot!(dump(
        "empty? paid? sort! push!(4) a!=b a! = b x?? x!! foo()? xs[0]? 1? x ? \
         Some? Foo! self? result? task.await? resp.body? yield?"
    ));
}

#[test]
fn plain_strings_and_escapes() {
    insta::assert_snapshot!(dump(
        r#""hello" "" "tab\there" "quote \" inside" "back\\slash" "\#{not interp}" "\u{1F600} \u{e9}" "café €""#
    ));
}

#[test]
fn raw_strings() {
    insta::assert_snapshot!(dump(
        r###"r"C:\no\#{escapes}" r#"has "quotes""# r##"a "# inside"## r"" rx r #[attr]"###
    ));
}

#[test]
fn triple_quoted_strings() {
    insta::assert_snapshot!(dump(
        "let s = \"\"\"\n  line one\n  say \"hi\" and \"\"two\"\"\n  total #{n}\n  \"\"\"\nx"
    ));
}

#[test]
fn interpolation() {
    // `r##"..."##` because the input contains `"#`, which would end `r#"..."#`.
    insta::assert_snapshot!(dump(r##""a #{x} b #{y + 1} c" "#{only}" "#{a}#{b}""##));
}

#[test]
fn nested_interpolation() {
    // Braces inside the interpolation, and a string inside an interpolation
    // inside a string.
    insta::assert_snapshot!(dump(
        r#""n=#{ xs.map { |x| x } }" "outer #{ f("inner #{ g(1) }") } end""#
    ));
}

#[test]
fn unterminated_strings() {
    insta::assert_snapshot!(dump("puts(\"hello)\nnext_line"));
    insta::assert_snapshot!("unterminated_triple", dump("\"\"\"never\nclosed"));
    insta::assert_snapshot!("unterminated_raw", dump("r#\"no close\"\nx"));
}

#[test]
fn invalid_escapes() {
    insta::assert_snapshot!(dump(
        r#""\q" "\u{}" "\u{110000}" "\u{D800}" "\u{1234567}" "\u41" "\u{12" "ok\"#
    ));
}

#[test]
fn unterminated_interpolations() {
    // The documented PT0007 example: the `}` was forgotten.
    insta::assert_snapshot!(dump(r#"puts("total: #{n")"#));
    insta::assert_snapshot!("interp_hits_newline", dump("\"a #{x\ny"));
    insta::assert_snapshot!("interp_hits_eof", dump("\"\"\"a #{x"));
}

// --- Targeted behavior ---------------------------------------------------

/// Every program in `examples/` must lex without a single diagnostic.
#[test]
fn all_examples_lex_cleanly() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut checked = 0;
    let mut pending = vec![dir];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("examples directory exists") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "plat") {
                let text = std::fs::read_to_string(&path).expect("readable example");
                let lexed = lex_str(&text);
                assert!(
                    lexed.diagnostics.is_empty(),
                    "{} has lexer diagnostics: {:#?}",
                    path.display(),
                    lexed.diagnostics
                );
                checked += 1;
            }
        }
    }
    // 5 runnable examples plus 3 in examples/broken/ (broken at later
    // phases, but lexically valid).
    assert!(checked >= 8, "expected all examples, checked {checked}");
}

/// The text of each non-Eof token, for compact assertions.
fn texts(src: &str) -> Vec<String> {
    lex_str(src)
        .tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .map(|t| src[t.span.start as usize..t.span.end as usize].to_string())
        .collect()
}

#[test]
fn suffix_rule_cases() {
    assert_eq!(texts("empty?"), ["empty?"]);
    assert_eq!(texts("sort!"), ["sort!"]);
    assert_eq!(texts("a!=b"), ["a", "!=", "b"]);
    assert_eq!(texts("a!==b"), ["a", "!=", "=", "b"]);
    assert_eq!(texts("x??"), ["x?", "?"]);
    assert_eq!(texts("x?!"), ["x?", "!"]);
    assert_eq!(texts("x !"), ["x", "!"]);
    assert_eq!(texts("self?"), ["self", "?"]);
    assert_eq!(texts("Some?"), ["Some", "?"]);
    assert_eq!(texts("f()?"), ["f", "(", ")", "?"]);
    assert_eq!(texts("xs.empty?"), ["xs", ".", "empty?"]);
}

#[test]
fn reserved_word_with_suffix_is_still_reported() {
    let lexed = lex_str("yield?");
    assert_eq!(lexed.diagnostics.len(), 1);
    assert_eq!(lexed.tokens[0].kind, TokenKind::LowerIdent);
}

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
        "\"",
        "\"\"\"",
        "\"\"\"\"",
        "\"#{",
        "\"#{\"",
        "\"#{\"#{",
        "\"\\",
        "\"\\u{",
        "\"\\u{€}\"",
        "r#",
        "r#\"",
        "r##\"\"#",
        "\"€#{€}€\"",
    ] {
        inputs.push(s.to_string());
    }
    // Every string up to length 5 over the characters that matter to string
    // lexing (59,049 + shorter inputs), to shake out mode-stack bugs.
    let alphabet = ['"', '#', '{', '}', '\\', '\n', 'r', 'u', 'x'];
    let mut frontier = vec![String::new()];
    for _ in 0..5 {
        let mut next = Vec::new();
        for s in &frontier {
            for &c in &alphabet {
                let mut t = s.clone();
                t.push(c);
                next.push(t);
            }
        }
        inputs.extend(next.iter().cloned());
        frontier = next;
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
        if let Err(problem) = check_string_structure(tokens) {
            panic!("unbalanced string tokens for {src:?}: {problem}\n{tokens:#?}");
        }
    }
}

/// Checks that string pieces form a well-nested sequence, which the parser
/// will rely on:
///
/// ```text
/// StrStart InterpStart ...code... InterpEnd (StrMiddle InterpStart ...code... InterpEnd)* StrEnd
/// ```
///
/// The "code" parts may themselves contain whole strings, so this keeps a
/// stack of open interpolations.
fn check_string_structure(tokens: &[crate::token::Token]) -> Result<(), String> {
    use TokenKind::*;
    // Each entry: are we waiting for the string piece after an InterpEnd?
    let mut open_interps = 0usize;
    let mut expect_interp_start = false;
    let mut expect_piece_after_interp = false;

    for (i, t) in tokens.iter().enumerate() {
        let k = t.kind;
        if expect_interp_start {
            if k != InterpStart {
                return Err(format!(
                    "token {i}: expected InterpStart after a string piece, got {k:?}"
                ));
            }
            expect_interp_start = false;
        }
        if expect_piece_after_interp {
            if !matches!(k, StrMiddle { .. } | StrEnd { .. }) {
                return Err(format!(
                    "token {i}: expected StrMiddle/StrEnd after InterpEnd, got {k:?}"
                ));
            }
            expect_piece_after_interp = false;
        }
        match k {
            StrStart { .. } | StrMiddle { .. } => expect_interp_start = true,
            InterpStart => open_interps += 1,
            InterpEnd => {
                if open_interps == 0 {
                    return Err(format!("token {i}: InterpEnd with no open interpolation"));
                }
                open_interps -= 1;
                expect_piece_after_interp = true;
            }
            _ => {}
        }
    }
    if open_interps != 0 || expect_interp_start || expect_piece_after_interp {
        return Err("stream ended with an unclosed string or interpolation".to_string());
    }
    Ok(())
}
