# CLAUDE.md: building Platinum

You are implementing **Platinum**, a programming language specified in this repository. Read `README.md`, then `spec/00-overview.md`, then the spec file for whatever milestone you are on. `ROADMAP.md` defines the order of work. `DECISIONS.md` explains why things are the way they are; do not silently reverse a decision. If the spec is ambiguous or wrong, propose a change in `DECISIONS.md` (new entry, status "proposed") and ask Brian.

## Who you are working with

Brian is a software engineer (UCF CS, works on payments API integrations, writes C#/.NET and Java, learning Rust through this project). **This is a learning project as much as a build.** That changes how you work:

- Work one milestone at a time. Within a milestone, keep each change small enough to review in a few minutes.
- Before starting a milestone, write a short plan in `notes/Mx-plan.md`: what you will build, the concept it teaches, and the 2 or 3 key design choices. Ask before proceeding if any choice is not already settled by the spec.
- Every non-trivial module gets a top-of-file doc comment explaining the concept (for example, what a Pratt parser is and why we use it) in plain language, with a link from `LEARNING.md` if one exists.
- At the end of a milestone, write `notes/Mx-retro.md`: what was built, what was surprising, what Brian should read or try next. Then stop and summarize.
- Prefer clear code over clever code. No macros-heavy abstractions unless they remove real repetition.
- Writing style for all docs and comments: do not use em-dashes; use commas, periods, colons, or parentheses.

## Repository layout to create (Cargo workspace)

```
platinum/
  Cargo.toml                  # workspace
  crates/
    plat-syntax/              # lexer, parser, AST, spans, pretty printer
    plat-diag/                # Diagnostic type, codes, JSON + human rendering, fix plans
    plat-hir/                 # name resolution, desugaring, lowered IR, definition hashing
    plat-types/               # bidirectional type checker, traits, exhaustiveness
    plat-effects/             # capability inference and checking
    plat-verify/              # refinement + contract VC generation, Z3 backend, counterexamples
    plat-interp/              # M1-M4 tree-walking interpreter
    plat-vm/                  # M6 bytecode compiler + VM + Perceus RC
    plat-codegen/             # M8 Cranelift backend
    plat-cli/                 # `plat` binary
    plat-mcp/                 # MCP server (`plat mcp`)
    plat-lsp/                 # language server (`plat lsp`)
  std/                        # Platinum standard library sources (.plat)
  tests/
    ui/                       # golden tests: foo.plat + foo.stdout / foo.diag.json
  examples/                   # from this spec repo
  spec/ ROADMAP.md DECISIONS.md LEARNING.md   # keep these, update as you go
  notes/                      # milestone plans and retros
```

## Crate choices (use these unless there is a strong reason not to)

| Need | Crate | Note |
|---|---|---|
| Lexing | hand-written lexer (preferred) or `logos` | Hand-written is better for learning and for the `?`/`!` identifier rule |
| Parsing | hand-written recursive descent + Pratt for expressions | Must recover from errors and keep going (multiple diagnostics per run) |
| Pretty diagnostics | `ariadne` | Human renderer only; JSON is the source of truth |
| Serialization | `serde`, `serde_json` | Diagnostic schema in `spec/09-tooling.md` |
| Arbitrary precision ints | `num-bigint` | `Int` is unbounded |
| Decimal | `rust_decimal` (start) | Revisit for arbitrary precision later |
| SMT | `z3` crate (high-level bindings) | Gate behind a cargo feature `verify` so the rest builds without Z3 |
| Hashing | `blake3` | Definition hashes |
| Snapshot tests | `insta` | For AST, types, diagnostics |
| CLI | `clap` | Every subcommand supports `--json` |
| MCP | `rmcp` (official Rust MCP SDK) | Check the current version and API before using |
| LSP | `tower-lsp` | M5 |
| Native backend | `cranelift` family | M8 |
| Incremental queries | `salsa` | Optional, M5 or later; do not start with it |

## Ground rules

1. **Diagnostics first.** Every error the compiler can produce has a stable code (`PT0001` style, ranges in `spec/09-tooling.md`), a span, a message, and where possible a fix plan. Add the code to `plat-diag/src/codes.rs` with a one-paragraph explanation (used by `plat explain`).
2. **Golden tests for everything.** Each feature lands with `tests/ui/*.plat` files and expected stdout or expected JSON diagnostics. The examples in `examples/` must run (or produce their documented diagnostics) at the milestone where their features exist.
3. **The spec is the contract.** If implementation reveals the spec is wrong, update the spec in the same change and add a DECISIONS entry.
4. **No `unwrap()` on user-controlled data in the compiler.** Compiler panics are bugs; user errors are diagnostics.
5. **Spans everywhere.** Every AST and HIR node carries a span. Counterexamples and fix plans depend on it.
6. **Keep the interpreter alive.** After M6 the tree-walking interpreter stays as the reference semantics; the VM and native backends must match it on the golden suite.

## Commands to support from early on

- `cargo test` runs all unit and golden tests.
- `plat run file.plat`, `plat check file.plat [--json]`, `plat fmt` (can be minimal), `plat explain PT1003`.

## Start here

Begin with **M0** in `ROADMAP.md`.
