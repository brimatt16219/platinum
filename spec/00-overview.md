# 00. Overview

## Vision

Most bugs that reach production are broken promises: a function assumed its input was positive, a refund exceeded the payment, a library made a network call nobody expected. Platinum makes those promises part of the code and has the compiler check them.

As more code is written by AI agents, this matters more. An agent can produce plausible code quickly; what it lacks is a trustworthy signal that the code is right. Platinum's compiler is that signal: it proves contracts when it can, shows a concrete counterexample when it cannot, and reports everything in a structured form an agent can act on.

## The three pillars (the "truss")

| Pillar | What the programmer writes | What the compiler guarantees |
|---|---|---|
| **Types** | ADTs, records, traits, generics, `Int where self > 0` | No null, no unhandled variant, no type confusion, refinements hold |
| **Effects** | `needs Net, Clock` on public functions | A function cannot perform I/O it did not declare; tests can replace any capability |
| **Contracts** | `requires`, `ensures`, `invariant`, `assert` | Proven by Z3 at compile time, or refuted with a counterexample, or (by explicit choice) checked at runtime |

## Design principles

1. **Small.** Fewer concepts than Rust or Scala. If a feature can be library code, it is.
2. **Explicit at boundaries, inferred inside.** Public signatures carry types, effects, and contracts. Function bodies infer almost everything.
3. **Gradual verification.** Code with zero contracts compiles and runs. Add a contract and it gets proven. Nothing forces formal methods on you.
4. **Errors are values.** `Result`, `?`, tag unions. No exceptions, no null.
5. **Joy (the Ruby part).** Trailing blocks, predicate names (`empty?`), mutating variants (`sort!`), string interpolation, chain-everything method syntax, great error messages.
6. **Machine-legible.** Every tool emits JSON. Every diagnostic has a stable code. The grammar is unambiguous and the formatter is canonical, so agents produce one style.
7. **Money-safe numbers.** `1.50` is an exact `Decimal`, not a float. `Int` never overflows silently.

## Non-goals (for v1)

- Systems programming (manual memory, raw pointers, inline assembly).
- Full dependent types or interactive theorem proving.
- Object-oriented inheritance.
- Macros. (Revisit after v1.)
- Proving termination. Contracts are partial correctness in v1; `decreases` is reserved.

## Target users

1. Developers writing business logic where correctness matters (payments, billing, permissions, workflows).
2. AI coding agents working in a generate, check, repair loop.
3. Learners who want to understand types, effects, and verification by using them.

## Competitive landscape (as of November 2026)

| Language | Overlap | How Platinum differs |
|---|---|---|
| **Vow** (vow-lang, 2026) | Agent-first, contracts verified before shipping, Rust + Cranelift | Vow is an imperative systems language that explicitly gives up human readability and uses bounded model checking (ESBMC). Platinum is functional, human-readable, uses refinement *types* plus SMT, and tracks effects as capabilities. |
| **Zero** (Vercel Labs, 2026) | Agent-first, JSON diagnostics with stable codes and repair plans, effects in stdlib signatures | Zero is a systems language focused on agent-legible tooling. Platinum adopts that tooling as table stakes and adds compile-time proofs of user contracts. |
| **Dafny** | Contracts verified with Z3 | Dafny targets verification experts and compiles to other languages. Platinum aims to feel like an everyday language with verification as an opt-in layer, plus effects. |
| **Koka / Effekt** | Effect systems, Perceus (Koka) | Research languages without refinement types or agent tooling. Platinum borrows Perceus (Koka) and the capability reading of effects (Effekt). |
| **Roc / Gleam** | Small functional languages, no runtime exceptions | No verification layer. Platinum borrows Roc's tag unions for errors and Gleam's commitment to a small surface. |
| **Liquid Haskell / Flux** | Refinement types via SMT | Add-ons to existing languages. Platinum builds refinements into the core. |
| **Unison** | Content-addressed code | Platinum adopts definition hashing for caching and semantic diffs (later stages), not the full codebase-as-database model in v1. |

**The defensible niche:** the only language that is simultaneously (a) readable by humans, (b) proven by its compiler, (c) explicit about effects, and (d) built around an agent repair loop.

## Implementation strategy

Staged, each stage shippable (details in `ROADMAP.md`):

1. Tree-walking interpreter (reference semantics, fastest feedback).
2. Bytecode VM with Perceus reference counting.
3. Native code via Cranelift.

The compiler is written in Rust.
