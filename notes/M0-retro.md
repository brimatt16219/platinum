# M0 retro: Foundations

Finished 2026-11-26, in 13 steps (one commit each, plus decision commits).

## What was built

| Crate | Contents |
|---|---|
| `plat-diag` | `Span`, `SourceFile` (line table, character columns), `SourceMap`, `Diagnostic` / `Label` / `Fix` / `Edit`, `DiagnosticBag`, the code registry (PT0001 to PT0008 with explanations), JSON renderer (`platinum.diagnostics/v1`), human renderer (ariadne) |
| `plat-syntax` | `Token` / `TokenKind` / `Keyword`, and the complete lexer: comments (nested, doc), identifiers with the `?`/`!` rule, keywords and reserved words, every number form, operators by maximal munch, all three string forms with nested interpolation, escapes, and the newline rule |
| `plat-cli` | `plat check [--json] [--color]`, `plat explain [--json]`, stubs for `run` and `fmt`; exit codes 0 / 1 / 2 |
| `tests/ui/lex/` | 11 golden tests, a harness with bless mode, and a check that every lexer code has a test |

Numbers: about 3,800 lines of Rust in `src/` (5,200 with tests), 107 tests (unit, snapshot, doc, CLI integration, golden), 31 reviewed snapshots. The robustness test lexes about 66,000 generated inputs on every `cargo test` and checks that the lexer never panics and its output is always well formed.

All three ROADMAP "done when" criteria hold: `plat check --json` reports lexical errors in the v1 schema, every token kind has snapshot coverage, and `plat explain PT0001` works.

## Decisions made

- **D-17 to D-20** (planned): newline rule in the lexer, interpolation as a token stream with a mode stack, character columns, code sub-ranges.
- **D-21** (found in step 6): the newline rule as written broke the spec's own examples. Added `|` and `->` as line continuers, and gave the parser the job of skipping newlines inside item headers. Step 10 found two more places for the parser (attribute lines, brace-delimited lists).
- **D-22** (step 7): number literal edge cases, including the `pair.0.1` tuple rule.
- **D-23** (step 9): string edge cases; only `"""` strings span lines.

## What was surprising

1. **The spec's examples were the best test of the spec.** D-21 exists because `examples/bank.plat` and `examples/shapes.plat` could not be lexed under D-17 as written. Lexing real programs early (the `all_examples_lex_cleanly` test) found problems that reading the spec did not.
2. **Raw string delimiters bit twice.** Both a registry explanation (step 4) and a test (step 9) contained `"#`, which ends Rust's `r#"..."#` early. Platinum's raw strings have the same rule, so this is now a lesson baked into the lexer too.
3. **Library behavior is only knowable from source.** ariadne silently draws nothing for a label without text, and splits a report when labels go backward in line order. Both were found by reading its source after a snapshot looked wrong.
4. **Line endings are part of the data.** With `core.autocrlf=true`, a fresh clone would have shifted every byte offset and broken golden tests on any other machine. `.gitattributes` now pins LF.
5. **Small consistency tests pay off.** "Every explanation has an example" caught drift on its first run; "checked at least N examples" caught a wrong assumption about how many there are.
6. **Error recovery is mostly about one case.** The single heuristic for `puts("total: #{n")` (a quote inside an interpolation with no `}` left on the line ends the string) turns a cascade of confusing errors into exactly one correct one.

## Carried into M1 (the parser's obligations)

- Skip `Newline` tokens in item headers (`fn` to `{`, loop heads, around `=` in `type`), between an attribute and its item, and inside brace-delimited lists (D-21 and its note).
- Accept `OrOr` (`||`) as the start of a zero-parameter lambda.
- Decode escapes and strip the indentation of `"""` strings (D-23 item 6); compute number values (`num-bigint`, `rust_decimal`).
- Treat an empty `StrMiddle` (between `#{a}#{b}`) as normal.
- Decide the formatter rules (open question Q-05) before `plat fmt`.
- Later (M2): the resolver's `name?` to `(name)?` fallback, and lint PT1041.

Also outstanding: the GitHub Pages docs-site session was opened but never received its brief (the permission classifier blocked it). It needs to be restarted with a task description.

## What to read or try next

1. **Read the code in order:** the table in `LEARNING.md` ("Read the M0 code") goes from spans up to the CLI. Pay special attention to `push` and `pending_newline` in `lexer.rs`, the one piece of lookahead in the lexer.
2. **Crafting Interpreters, chapters 5 to 8** (representing code, parsing expressions, evaluating, statements). Chapter 4 (scanning) maps onto what M0 built; compare its design with ours.
3. **Pratt parsing:** Bob Nystrom, "Pratt Parsers: Expression Parsing Made Easy", and matklad, "Simple but Powerful Pratt Parsing". M1's expression parser will follow the second closely.
4. **Do the M1 exercise in `LEARNING.md`:** hand-parse `a + b * c |> f` with the precedence table in `spec/02-grammar.md` before reading the parser.
5. **Try it yourself:**
   - add a diagnostic end to end: a lexer warning for a tab character inside an identifier line, say. Add the code and explanation in `codes.rs`, report it in the lexer, add `tests/ui/lex/<case>.plat`, bless it, and watch the coverage test enforce the golden file;
   - run `plat check --json` on a file you write by hand and read the JSON an agent would see;
   - install `cargo-insta` and use `cargo insta review` the next time a snapshot changes.
