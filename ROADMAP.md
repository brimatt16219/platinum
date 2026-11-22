# Roadmap

Each milestone is shippable and teaches one layer of language implementation. Do them in order. "Done when" items are acceptance criteria; each should be backed by golden tests in `tests/ui/`.

## Stage 1: Interpreter (reference semantics)

### M0: Foundations
Build: Cargo workspace, `plat-diag` (Diagnostic type, codes registry, JSON and `ariadne` rendering), `plat-syntax` lexer (including the `?`/`!` identifier rule and newline termination), spans, `plat explain`.
Done when: `plat check --json file.plat` reports lexical errors in the v1 schema; lexer has snapshot tests for every token kind; `plat explain PT0001` works.
Learn: tokens, spans, why diagnostics are data.

### M1: Parser and tree-walking interpreter (untyped)
Build: full recursive-descent parser with Pratt expressions and error recovery; AST pretty printer; minimal `plat fmt`; interpreter for literals, `let`/`var`, functions, closures, trailing blocks, records, variants, `match`, `if`, loops, lists, string interpolation, `?`; `plat run`, `plat repl`. Effects not checked yet: built-in capability operations simply work.
Done when: `examples/hello.plat` and `examples/shapes.plat` run; parser reports multiple errors in one file; `fmt` is idempotent on all examples.
Learn: grammars, precedence climbing, ASTs, environments and closures, evaluation.

### M2: Type checker
Build: `plat-hir` (name resolution, desugaring of methods, `!` methods, `?`, blocks), bidirectional checker with unification, generics, records, variants, tag unions with width subtyping, traits and impls (dictionary passing), exhaustiveness checking, numeric rules, `plat check`, first fix plans (missing match arms, missing imports, `let` to `var`). Definition hashing (for cache keys).
Done when: all examples type check; the deliberately broken files in `tests/ui/types/` produce the documented codes and fix plans.
Learn: type inference, unification, bidirectional typing, typeclasses, pattern exhaustiveness.

### M3: Effects and capabilities
Build: `plat-effects` (inference over the call graph, `needs` checking, `pub` boundary rule, block vs closure rules), `cap` declarations, `handle ... with`, implicit capability passing in the interpreter, `test` blocks, `plat test`, `std.test` fakes.
Done when: `examples/refund.plat` tests pass with `FakeGateway`; removing a `needs` produces `PT2001` with a correct fix; a closure stored in a list that uses `Net` without declaring it is rejected.
Learn: effect systems, capabilities, dependency injection as a language feature.

### M4: Verification
Build: `plat-verify`: VC generation (path-sensitive), encoding to Z3, measures, `#[reflect]`, counterexample extraction and mapping to source, outcome policy, `#[runtime_check]`, `plat prove`, "add test from counterexample" fix.
Done when: `examples/bank.plat` proves; introducing the classic off-by-one yields `PT3001` with the counterexample shown in `05-verification.md`; proof results are cached by definition hash.
Learn: SMT solving, verification conditions, Hoare logic, refinement types, why value semantics makes this tractable.

### M5: Agent and editor tooling
Build: `plat-mcp` (all tools in `09-tooling.md`, spec served as resources), `plat-lsp` (diagnostics, hover, go-to-def, code actions), `plat fix --apply`.
Done when: an agent (Claude Code configured with `plat mcp`) can take `examples/broken/` files from failing to proven using only MCP tools; VS Code shows hovers with refinements and capabilities.
Learn: MCP, LSP, designing tools for machine consumers.

## Stage 2: Bytecode VM

### M6: VM and Perceus
Build: lower typed HIR to a mid-level IR (A-normal form with explicit control flow); Perceus pass (`dup`/`drop` insertion, drop-guided reuse); register- or stack-based bytecode (write a DECISIONS entry); VM in Rust; monomorphization of generics; `plat build`. The interpreter remains the oracle: the full golden suite must produce identical output on the VM.
Done when: golden suite passes on both; a benchmark (e.g. red-black tree insert, list sort) shows the VM at least 10x faster than the interpreter; refcount debug mode proves no leaks on the suite.
Learn: IR design, bytecode, VMs, reference counting, reuse analysis.

### M7: Structured concurrency
Build: `scope`/`spawn`/`await`, cancellation, `Channel`, deterministic cooperative scheduler (interpreter and VM), `--seed`.
Done when: `examples/dashboard.plat` runs; a panicking child cancels siblings; tests are reproducible by seed.
Learn: schedulers, cancellation, structured concurrency.

## Stage 3: Native

### M8: Cranelift backend
Build: lower the M6 IR to Cranelift IR; runtime library in Rust (allocation, refcounts, strings, bigint, decimal); native executables.
Done when: golden suite passes natively; benchmarks recorded against the VM.
Learn: SSA, codegen, calling conventions, runtime libraries.

### M9: Content-addressed codebase features
Build: `plat diff --semantic`, proof certificates keyed by hash, hash-based dependency resolution, `plat deps audit` (capability changes between versions).
Done when: renaming a function and reformatting a file yields an empty semantic diff; a dependency upgrade that adds `needs Net` is flagged.
Learn: content addressing, Merkle structures, supply-chain security.

## Later (post v1 ideas, not committed)

- Liquid-style invariant inference (Horn clause solving).
- Termination checking with `decreases`.
- Full algebraic effects with `resume` (generators, async as library).
- Self-hosting the compiler in Platinum.
- WASM target.
- Property-based testing that uses the verifier's counterexamples as seeds.
