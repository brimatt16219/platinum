# M0 plan: Foundations

Status: **done** (2026-11-26). Open choices A to D resolved with the recommended options (DECISIONS D-17 to D-20); implementation added D-21 to D-23. See `notes/M0-retro.md`.

## Goal (from ROADMAP.md)

Build the Cargo workspace, `plat-diag` (Diagnostic type, codes registry, JSON and `ariadne` rendering), the `plat-syntax` lexer (including the `?`/`!` identifier rule and newline termination), spans, and `plat explain`.

**Done when:**
- `plat check --json file.plat` reports lexical errors in the v1 diagnostic schema (`spec/09-tooling.md`).
- The lexer has `insta` snapshot tests for every token kind.
- `plat explain PT0001` prints the long explanation.

## Concepts this milestone teaches

1. **Tokens vs lexemes.** A lexeme is the raw text (`1_000.50`); a token is its classification (`Decimal`) plus where it came from. The lexer's job is to classify, not to compute values.
2. **Spans.** Every token carries a byte range into the source. Line and column are *derived* from the byte offset on demand, never stored per token. Everything later (parser errors, counterexamples, fix plans) points back through spans.
3. **Diagnostics are data.** A `Diagnostic` is a plain struct (code, severity, message, labeled spans, notes, fixes). Human output (ariadne) and JSON output are two renderers over the same struct. JSON is the source of truth.
4. **Rust basics in context:** `enum` + `match` for token kinds, `struct` with `serde::Serialize`, a Cargo workspace, `clap` for the CLI, snapshot testing with `insta`.

## What gets built, in reviewable steps

Each step should be small enough to review in a few minutes.

0. **Prerequisite:** install Rust via `rustup` (stable toolchain). It is not on this machine yet. `git init` the repo so each step is a commit.
1. **Workspace skeleton.** Root `Cargo.toml` with `members = ["crates/*"]`; create `plat-diag`, `plat-syntax`, `plat-cli` (binary named `plat`). Only these three crates in M0; the others arrive with their milestones. `cargo test` passes on empty crates.
2. **Spans and source files** (`plat-diag/src/span.rs`, `source.rs`).
   - `Span { start: u32, end: u32 }` (byte offsets, half-open).
   - `SourceFile { name, text, line_starts: Vec<u32> }` with `line_col(offset)` using binary search over `line_starts`.
   - Unit tests for line/col at line boundaries, CRLF, and multibyte characters.
3. **Diagnostic type** (`plat-diag/src/diagnostic.rs`).
   - `Diagnostic { code, severity, message, primary: Label, secondary: Vec<Label>, notes, fixes }`, mirroring the v1 JSON schema field for field (including `fixes` with `applicability`, even though M0 lexer errors will rarely have one).
   - `DiagnosticBag` (a `Vec<Diagnostic>` with helpers) so every pass collects errors instead of stopping at the first.
4. **Codes registry** (`plat-diag/src/codes.rs`).
   - A static table: code, short title, long explanation (one paragraph plus a small example). Used by `plat explain`.
   - M0 lexer codes (proposed, PT0001 to PT0099 reserved for lexing, PT0100+ for parsing):
     - `PT0001` unexpected character
     - `PT0002` unterminated string literal
     - `PT0003` unterminated block comment
     - `PT0004` invalid escape sequence
     - `PT0005` malformed number literal (e.g. `0x`, `1__0`, `1.5e`)
     - `PT0006` unknown numeric suffix (e.g. `42i7`)
     - `PT0007` unterminated string interpolation `#{`
     - `PT0008` use of reserved keyword (`async`, `decreases`, `macro`, `unsafe`, `yield`)
   - A unit test asserting every code is unique, in its range, and has a non-empty explanation.
5. **Renderers** (`plat-diag/src/render/json.rs`, `human.rs`).
   - JSON: the full v1 envelope (`schema`, `tool`, `summary`, `diagnostics`), via `serde`. Snapshot-tested.
   - Human: `ariadne` report from the same struct.
6. **Token kinds** (`plat-syntax/src/token.rs`). One `enum TokenKind` covering keywords, identifiers (lower/upper, with absorbed `?`/`!`), literals (Int, sized int, Decimal, Float, String parts, raw, multiline, Bool), every operator and punctuation in `spec/01-lexical.md`, `Newline`, `Eof`, and `Error`. Tokens store the kind and span; literal values are parsed later (keeps the lexer simple and lossless).
7. **Lexer core** (`plat-syntax/src/lexer.rs`). Hand-written, one `Cursor` over `&str` with `peek`, `peek2`, `bump`. Covers whitespace, all three comment forms (with nested `/* */`), identifiers and keywords, numbers, operators. Never panics on any input; bad input produces an `Error` token plus a diagnostic, then continues.
8. **The `?`/`!` rule.** `empty?` absorbs, `a!=b` does not, `foo()?` is the propagation operator. The resolver fallback (`name?` to `(name)?`) is an M2 concern; the lexer only implements the absorption rule from `spec/01-lexical.md`.
9. **Strings and interpolation.** Escapes, raw strings, `"""` multiline with indentation stripping, and `#{ ... }` (see open choice B).
10. **Newline termination** (see open choice A).
11. **CLI** (`plat-cli`): `plat check [--json] <file>` (lex only for now, exit code 1 on errors), `plat explain <CODE> [--json]`. `plat run` and `plat fmt` are stubs that print "not implemented until M1".
12. **Golden tests** (`tests/ui/lex/*.plat` with expected `.diag.json`), plus lexing all of `examples/` with zero diagnostics.
13. **Docs:** top-of-file concept comments on `span.rs`, `diagnostic.rs`, `lexer.rs`; link them from `LEARNING.md`; write `notes/M0-retro.md`.

## Key design choices

Settled by the spec or CLAUDE.md (not asking):
- Hand-written lexer, not `logos`.
- JSON diagnostics are the source of truth; ariadne is a view.
- Literal values (bigint, decimal) are not computed in the lexer; `num-bigint` and `rust_decimal` come in M1 when the parser/interpreter need values.
- `result` and `self` are lexed as keywords but treated contextually by the parser.

Resolved by Brian (2026-11-21): A = lexer, Go style; B = token stream with mode stack; C = Unicode-character columns; D = PT0001-0099 lexing, PT0100-0999 parsing. The options as originally presented:

**A. Where does the newline rule live?** The spec says a newline ends a statement when "the line could syntactically end there and the next line does not begin with a binary operator, `.`, `|>`, or `?`".
- *Option 1 (recommended): lexer, Go style.* The lexer emits `Newline` only if the previous token can end a statement (identifier, literal, `)`, `]`, `}`, `?`, `return`, `break`, `continue`, ...) **and** the next non-blank token is not a continuation operator. The parser then just sees `Newline` as a terminator. Simple, testable in M0, and matches the M0 roadmap item.
- *Option 2: parser decides.* The lexer emits every newline; the parser skips them where an expression is incomplete. More flexible, but spreads newline logic through the whole parser.
- Also: newlines inside `( )` and `[ ]` are always ignored under either option (proposed; the spec does not say).

**B. How to lex string interpolation `"a #{x + 1} b"`?**
- *Option 1 (recommended): token stream with a mode stack.* Lexer emits `StrStart("a ")`, `InterpStart`, then normal tokens for `x + 1`, `InterpEnd`, `StrEnd(" b")`. A stack of modes handles nested braces and strings inside interpolations. The parser sees ordinary expression tokens; spans inside interpolations are exact.
- *Option 2: lex the whole string as one token, re-lex the `#{}` parts in the parser.* Easier lexer, but spans need offset juggling and errors inside interpolations get worse.

**C. What unit is the "col" in diagnostics?** JSON has `line`, `col`, `offset`.
- *Option 1 (recommended):* 1-based line, 1-based column counted in Unicode characters; `offset` in bytes. The LSP (M5) converts to UTF-16 at its boundary.
- *Option 2:* column in UTF-8 bytes (simplest, but wrong-looking for `"€"` and emoji).
- Whichever we pick gets written into `spec/09-tooling.md` with a DECISIONS entry.

**D. Diagnostic code ranges inside PT0xxx.** Proposed: `PT0001`-`PT0099` lexing, `PT0100`-`PT0999` parsing (consistent with the existing `PT0110` for `=>` misuse). Would be recorded in `spec/09-tooling.md`.
