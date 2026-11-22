# Learning guide

What to understand at each milestone, and where to read about it. Read the "core" item before starting a milestone; the rest when curious.

## M0 to M1: Lexing, parsing, interpreting

- **Core:** *Crafting Interpreters* by Robert Nystrom (free online at craftinginterpreters.com). Part II (tree-walk interpreter) maps almost exactly onto M1. Read the chapters on scanning, parsing expressions (Pratt parsing is in Part III, "Compiling Expressions"), and closures.
- Concepts: tokens vs lexemes; spans; recursive descent; operator precedence and Pratt parsing; error recovery (synchronize on `}` and newlines); environments; closures capturing variables.
- Exercise: before reading the parser code Claude writes, hand-parse `a + b * c |> f` into a tree using the precedence table in `spec/02-grammar.md`.

## M2: Type checking

- **Core:** Dunfield and Krishnaswami, "Bidirectional Typing" (survey): https://arxiv.org/pdf/1908.05839.pdf
- Concepts: type synthesis vs checking; unification variables; occurs check; let-generalization; typeclass dictionary passing; exhaustiveness via pattern matrices.
- Background book: Benjamin Pierce, *Types and Programming Languages* (chapters on simply typed lambda calculus, subtyping, and type reconstruction).
- Practical view: "How to choose between Hindley-Milner and bidirectional typing" (thunderseethe.dev, Feb 2026), summarized here: https://fooqux.com/article/2768
- Exercise: write by hand the constraints generated for `fn twice(f, x) { f(f(x)) }` and solve them.

## M3: Effects and capabilities

- **Core:** Brachthäuser, Schuster, Ostermann, "Effekt: Lightweight Effect Polymorphism for Handlers": https://ps.cs.uni-tuebingen.de/publications/brachthaeuser20effekt.pdf (read section 1 for the "capabilities a computation requires from its context" idea and second-class blocks).
- Koka overview (LWN, 2025): https://lwn.net/Articles/1033050
- Roc's platforms (another take on keeping I/O out of app code): https://roc-lang.org/platforms
- Concepts: effect rows vs capability sets; why second-class blocks avoid effect polymorphism; dependency injection; ambient authority vs capabilities.

## M4: Verification

- **Core:** the refinement types chapter of the Typechecker Zoo: https://sdiehl.github.io/typechecker-zoo/refinement/overview.html
- Flux: Liquid Types for Rust: https://arxiv.org/abs/2207.04034 (how refinements interact with mutation; Platinum avoids most of that with value semantics).
- "Refinement-Types Driven Development" (Liquid Haskell for everyday programming): https://arxiv.org/html/2509.15005v1
- Z3 from Rust, beginner guide: https://z3prover.github.io/papers/z3_rust_guide.pdf
- Why this matters for agents: LLM-generated Dafny annotations with verifier feedback loops: https://arxiv.org/html/2601.12845v1
- Concepts: Hoare triples; weakest preconditions; verification conditions; SAT vs SMT; theories (linear integer arithmetic, datatypes, uninterpreted functions); models as counterexamples; decidability and why we restrict the logic.
- Exercise: write the VC for `withdraw` in `examples/bank.plat` by hand, then feed it to Z3 directly (z3 accepts SMT-LIB text) and watch it say `unsat`; then introduce the off-by-one and read the model.

## M5: Tooling

- MCP specification and the Rust SDK (`rmcp`) docs.
- Vercel's Zero, for agent-oriented diagnostics design: https://betterstack.com/community/guides/ai/zero-language/
- LSP specification; `tower-lsp` examples.
- Concepts: designing stable machine interfaces; versioned schemas; fix applicability levels.

## M6: VM and memory

- **Core:** Perceus paper: https://www.microsoft.com/en-us/research/publication/perceus-garbage-free-reference-counting-with-reuse/
- Crafting Interpreters Part III (bytecode VM in C) for VM structure.
- Follow-up: "Reference Counting with Frame Limited Reuse": https://www.microsoft.com/en-us/research/uploads/prod/2023/07/flreuse.pdf
- Contrast: Hylo's mutable value semantics: https://github.com/hylo-lang/hylo-lang.github.io/blob/main/index.md, and Vale's generational references discussion: https://discourse.julialang.org/t/vales-generational-references-memory-management-and-higher-raii-vs-julia-and-other-languages/114457
- Concepts: A-normal form; linear resource calculus; dup/drop; reuse tokens; monomorphization; stack vs register VMs.

## M7: Concurrency

- **Core:** Nathaniel J. Smith, "Notes on structured concurrency, or: Go statement considered harmful" (vorpus.org).
- Design space of async/await (Brown PLT, Sept 2026): https://cel.cs.brown.edu/blog/design-space-async-await
- Concepts: nurseries/scopes; cancellation; cooperative scheduling; deterministic testing.

## M8: Native code

- Cranelift documentation and its IR reference.
- Concepts: SSA form, basic blocks, calling conventions, linking a runtime library.

## M9: Content addressing

- Unison, "The big idea": https://unison-lang.org/docs/the-big-idea
- Concepts: Merkle hashing of ASTs; alpha-equivalence; semantic diffs.

## Competitive reading (know your neighbors)

- Vow (agent-first, contracts via bounded model checking): https://github.com/vow-lang/vow
- Zero (agent-first diagnostics): see M5 link above.
- Gleam on staying small: https://spin.atomicobject.com/first-impressions-of-gleam/
