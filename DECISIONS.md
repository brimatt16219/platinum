# Design decisions

Format: each decision lists the choice, the alternatives considered, and why. Status is `accepted` unless marked otherwise. Add new entries at the bottom; never delete, supersede instead.

## D-01: Niche: an agent-era verified language
**Choice:** small, strictly typed language where contracts and refinements are proven at compile time and effects are capabilities, designed for humans and agents.
**Alternatives:** finance/business DSL; simple-but-safe systems language (Hylo/Vale style); content-addressed distributed language (Unison style).
**Why:** a real, timely gap. Research in 2026 shows LLMs with verifier feedback can produce correct Dafny annotations at very high rates, which means a language built around the generate, check, repair loop has a strong story. Systems and distributed niches are crowded or owned.

## D-02: Staged implementation (interpreter, then VM, then native)
**Alternatives:** spec plus interpreter only; native compiler from day one; transpile to C#, JS, or C.
**Why:** each stage is shippable and teaches one layer. The interpreter doubles as the reference semantics for testing later backends.

## D-03: Compiler written in Rust
**Alternatives:** C#/.NET, OCaml, Zig.
**Why:** strongest compiler ecosystem today (parsers, Z3 bindings, Cranelift, LSP, MCP SDK). Learning Rust's ownership model also informs Platinum's own memory design.

## D-04: Functional-first, pragmatic
**Alternatives:** pure functional; multi-paradigm with OOP; data-oriented procedural.
**Why:** immutable data and ADTs make verification and effect tracking tractable; local `var` and loops keep it approachable.

## D-05: Perceus reference counting
**Alternatives:** tracing GC; mutable value semantics (Hylo); ownership and borrowing (Rust).
**Why:** deterministic, no pauses, no lifetimes for users, and reuse analysis gives in-place performance for functional code. Value semantics means no cycles.

## D-06: Tiered verification (contracts plus refinements, proven by Z3, runtime fallback by explicit opt-in)
**Alternatives:** refinements only; full dependent types; runtime contracts only.
**Why:** expressive enough for real business rules, decidable fragment keeps the solver predictable, gradual so unannotated code still compiles.

## D-07: Effects as capabilities, tail-resumptive only in v1
**Alternatives:** full algebraic effects with resumable handlers; Roc-style platform-provided effects; untracked effects.
**Why:** signatures become honest and testable, supply-chain auditing falls out for free, and without resumptions the implementation is plain dictionary passing. Full algebraic effects remain a possible v2.

## D-08: Errors as values with `?` and structural tag unions
**Alternatives:** errors as an effect; exceptions; Go-style multiple returns.
**Why:** explicit control flow (good for verification and agents), and tag unions remove wrapper-enum boilerplate (borrowed from Roc).
**Note:** the early sketch wrote errors as `Err[OverLimit | Closed]`; the spec simplifies this to `Result[T, OverLimit | Closed]`.

## D-09: Brace syntax with Ruby flavor
**Alternatives:** Python-like indentation; ML/Haskell style; S-expressions; full Ruby `def ... end`.
**Why:** braces are unambiguous, robust to agent patching, and heavily represented in training data. Ruby touches (trailing blocks, `?`/`!` names, interpolation, chaining, paren-free zero-arg calls) keep it joyful. See D-14 for how the name reflects the Rust plus Ruby blend.

## D-10: Bidirectional type checking with traits; signatures required at top level
**Alternatives:** full HM with no traits; structural typing; explicit everywhere.
**Why:** local inference keeps bodies terse; required signatures give good errors, document intent for agents, and anchor refinement checking.

## D-11: Structured concurrency via the `Async` capability
**Alternatives:** actors; async/await; none in v1.
**Why:** no function coloring, no leaked tasks, errors always propagate. Deterministic scheduling in tests.

## D-12: Agent-native tooling from day one
**Choice:** JSON diagnostics with stable codes and fix plans; built-in MCP server; counterexamples on proof failure; content-addressed definition hashes.
**Why:** after Vercel's Zero (2026), JSON diagnostics are table stakes. Counterexamples and MCP are differentiators. Hashing starts as a cache key and grows into semantic diffs and dependency resolution.

## D-13: Numbers: arbitrary-precision `Int`, exact `Decimal` literals, explicit `Float`, opt-in sized ints
**Alternatives:** 64-bit `Int` by default; float-only numbers.
**Why:** no silent overflow, simple SMT encoding, and money is never a float by accident. `1.50` is a `Decimal`; floats need `f`.

## D-14: Name: Platinum, files `.plat`, CLI `plat`
**Why:** Platinum sits between its two parents: Rust is a metal (well, a metal's corrosion) and Ruby is a precious material, and platinum is a precious metal. The language takes from both: Rust's strictness, braces, `Result`/`?`, traits, and its implementation ecosystem; Ruby's blocks, `?`/`!` names, interpolation, chaining, and focus on developer happiness. `.pt` (the chemical symbol) was rejected as a file extension because PyTorch model files use it widely.

## D-15: Blocks are second-class; closures are first-class and carry their capabilities
**Why:** solves effect polymorphism without effect variables (the Effekt approach) and maps directly onto Ruby's block idiom. `each`, `map`, `retry` just work with effectful blocks.

## D-16: Lexer absorbs trailing `?`/`!` into identifiers; resolver fallback for `name?` propagation
**Alternatives:** Zig-style `try` prefix for propagation; disallow `?` in names.
**Why:** keeps both Ruby predicates and Rust-style propagation. The fallback rule is deterministic and documented in `01-lexical.md`. Revisit if it confuses agents in practice (track how often `PT1041` fires on agent-written code).

## D-17: Newline termination is decided in the lexer (Go style)
**Choice:** the lexer emits a `Newline` token only when the previous token can end a statement (identifier, literal, `)`, `]`, `}`, `?`, `return`, `break`, `continue`, `self`, `result`, `true`, `false`) and the next non-blank token is not a binary operator, `.`, `|>`, or `?`. Newlines inside `( )` and `[ ]` are never significant.
**Alternatives:** parser skips newlines wherever an expression is incomplete.
**Why:** keeps the parser's view simple (`Newline` is just a terminator), puts the whole rule in one testable place, and is a proven design (Go, Kotlin, Swift). Clarifies `spec/01-lexical.md`, which did not say which phase owns the rule or how brackets behave.

## D-18: String interpolation lexed as a token stream with a mode stack
**Choice:** `"a #{x} b"` lexes as `StrStart`, `InterpStart`, the tokens of `x`, `InterpEnd`, `StrEnd`. A mode stack handles nested braces and strings inside interpolations.
**Alternatives:** one string token, re-lexed by the parser.
**Why:** exact spans for errors inside interpolations, and the parser parses interpolated expressions with its ordinary expression code.

## D-19: Diagnostic columns count Unicode characters
**Choice:** `line` and `col` are 1-based; `col` counts Unicode scalar values; `offset` is a 0-based UTF-8 byte offset. The LSP converts to UTF-16 at its boundary.
**Alternatives:** byte columns; UTF-16 columns everywhere.
**Why:** columns match what a human sees in an editor, while byte offsets stay exact for edits and fix plans.

## D-20: Sub-ranges within PT0xxx
**Choice:** `PT0001`-`PT0099` lexing, `PT0100`-`PT0999` parsing.
**Why:** lets a code's number say which phase produced it; consistent with the existing `PT0110`.

## D-21: Newline rule amendments: `|` and `->` continue a line; newlines are insignificant in item headers
**Status:** accepted (2026-11-26, proposed in M0 step 6). Amends D-17.
**Problem:** D-17 as written breaks the spec's own examples.
1. Multi-line variant declarations (`spec/03-types.md`, `examples/shapes.plat`) start continuation lines with `|`: after `| Circle(radius: Float)` the line ends in `)`, so D-17 would end the statement before `| Rect(...)`.
2. `examples/bank.plat` (`transfer`) puts `-> Result[...]` on its own line after `)`.
3. Every function with contracts puts clauses and the body brace on their own lines (`examples/bank.plat`, `examples/refund.plat`): `-> Account` NL `ensures ...` NL `{`. After `Account` or after the `ensures` expression, D-17 emits a `Newline`, so the parser sees a terminator inside a function header.
**Choice:**
- (a) Lexer: add `|` and `->` to the continuation tokens (a line starting with either continues the previous line).
- (b) Parser (M1): `Newline` tokens are skipped inside item headers (between `fn` and the body's `{`, between `while`/`for` and the loop body, and before `=` / after `=` in `type` declarations), where a statement cannot end anyway.
**Alternatives:** make `needs`/`requires`/`ensures`/`invariant`/`{` continuation tokens in the lexer. Rejected for `{`: `foo()` NL `{ ... }` would silently become a trailing-block call, changing meaning. Rejected for the clause keywords as unnecessary once (b) exists, since (b) is needed for `{` anyway.
**Why:** (a) is a pure lexer fix that the examples require. (b) keeps the lexer rule simple (Go style) while allowing the brace-on-its-own-line layout the examples use, and limits newline tolerance to places where it cannot change meaning.

## D-22: Number literal details
**Status:** accepted (2026-11-26, proposed in M0 step 7). Clarifies `spec/01-lexical.md`, which lists literal forms by example but leaves edge cases open.
**Choice:**
1. `_` must sit between two digits: `1_000` is valid; `1_`, `1__0`, `0x_FF`, `1.5_` are `PT0005`.
2. An exponent (`e3`, `e-3`, `E+3`) is only allowed on a `Float`, so it needs the `f` suffix: `2.0e-3f`, `1e5f`. `1e5` is `PT0005` (Decimal literals are written out in full).
3. Digits followed directly by `f` are a `Float` even with no decimal point: `1f`.
4. Size suffixes (`i8` to `u64`) apply to decimal, hex, and binary integers (`0xFFu8`, `0b1i8`), but not to decimals (`1.5i32` is `PT0006`). `f` applies only to base-10 numbers (`0b1f` is `PT0006`; in hex, `f` is a digit).
5. Base prefixes are lowercase only (`0x`, `0b`); hex digits may be either case. There is no octal. Leading zeros are allowed (`007` is 7).
6. A `.` starts a fractional part only when a digit follows, so `1..5` and `1.max(2)` keep their `.`. After a `.` token the lexer never reads a fraction, so tuple access `pair.0.1` is `pair . 0 . 1`, not `pair . 0.1`.
**Alternatives:** Rust's rules (trailing `_` allowed, exponent on any float, `0o` octal). Rejected where they add forms without adding clarity; Platinum wants one way to write each number.
**Why:** each rule removes an ambiguity the spec left open, keeps literals readable, and matches what a reader would guess. Rule 6 avoids a known Rust papercut (`t.0.1` needing a space or parentheses).

## Open questions (to decide during implementation)

- **Q-01 (M6):** stack vs register bytecode.
- **Q-02 (M6/M7):** thread-safe refcounts (atomic everywhere vs per-task heaps with copy on send).
- **Q-03 (M2):** module system details: one module per file? visibility beyond `pub`? package manifest `plat.toml` format.
- **Q-04 (M4):** arbitrary-precision `Decimal` (replace `rust_decimal`) and how scale appears in refinements.
- **Q-05 (M1):** exact formatter rules (line width 100? trailing commas always in multiline?).
- **Q-06:** generic capabilities (e.g. `cap Store[K, V]`), useful but adds complexity.
