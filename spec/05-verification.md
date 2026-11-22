# 05. Verification: refinements and contracts

## What you can write

| Construct | Where | Meaning |
|---|---|---|
| `T where P` | any type position | values of `T` satisfying predicate `P` (`self` is the value) |
| `requires P` | fn clause | caller must establish `P` (parameters in scope) |
| `ensures P` | fn clause | function guarantees `P` on return (`result` is the return value; parameters in scope) |
| `invariant P` | `while` / `for` head | `P` holds before each iteration and after the loop |
| `assert P` | statement | `P` must be provable at this point |

Predicates are ordinary Platinum boolean expressions restricted to the **logic fragment** below, plus `=>` (implication).

```platinum
fn withdraw(acct: Account, amt: Int where self > 0) -> Result[Account, Insufficient]
  requires acct.open?
  ensures result.ok? => result.unwrap.balance == acct.balance - amt
  ensures result.ok? => result.unwrap.balance >= 0
{
  if amt > acct.balance { return Err(Insufficient) }
  Ok(Account { ..acct, balance: acct.balance - amt })
}
```

## The logic fragment (decidable by design)

Predicates may use:

- Literals of `Bool`, `Int`, `Decimal`, sized ints.
- Arithmetic `+ - *`, comparison, `&& || ! =>`, `==` on any `Eq` type that the encoder supports.
- Field access on records; constructor tests on variants (`x.ok?`, `match`-free tests generated per variant as `is_Circle?`).
- **Measures:** pure, total, structurally recursive functions marked `#[measure]`, such as `len`, `sum`, `keys`. Built-in measures: `List.len`, `String.len`, `Map.size`, `Set.size`, `Map.has?`, `Set.contains?`.
- Calls to `#[reflect]` functions: small pure non-recursive functions whose bodies are inlined into the logic.
- Quantifiers are **not** allowed in v1 surface syntax. (Use measures; for example `all_positive?(xs)` as a measure.)

Multiplication of two non-constant terms is allowed but may make the solver return `unknown` (nonlinear arithmetic). The diagnostic says so explicitly (`PT3004`).

## How checking works (for implementers)

This follows the Liquid Types / Flux approach: refinement checking reduces to **verification conditions (VCs)** discharged by an SMT solver.

1. After type checking, `plat-verify` walks typed HIR in a **path-sensitive** way, maintaining an environment of facts: parameter refinements, `requires` clauses, branch conditions (inside `if x > 0 { ... }` the fact `x > 0` holds), match constructor facts, and `let` equalities.
2. At each **obligation site** it emits a VC `facts => goal`:
   - passing an argument to a refined parameter (goal: the parameter's predicate),
   - calling a function (goal: its `requires`),
   - returning (goal: `ensures` and the return type's predicate),
   - constructing a record with refined fields,
   - fixed-width arithmetic (goal: no overflow), `Decimal` `/` exactness, list indexing (goal: `0 <= i < xs.len`),
   - loop invariants (initiation and preservation), `assert`.
3. Calls to other functions are **modular**: the callee's body is never inlined (except `#[reflect]`); only its signature's refinements and `ensures` are assumed.
4. Each VC is encoded to SMT-LIB terms via the `z3` crate: `Int` maps to SMT `Int`, `Decimal` to `Real` (with literals as exact rationals), sized ints to `Int` with range facts, records and variants to SMT algebraic datatypes, measures to uninterpreted functions with axioms generated from their definitions, `String` to the SMT string theory (equality and `len` only in v1).
5. Z3 is asked whether `facts && !goal` is satisfiable:
   - `unsat`: **Proven.**
   - `sat`: **Refuted.** Extract the model and map it back to source names: a **counterexample**.
   - `unknown` / timeout (default 2 s per VC, configurable): **Unknown.**

Loop invariants are user-supplied in v1 (no inference). A loop without an invariant contributes no facts after the loop except the negated condition.

## Outcomes and policy (tiered verification)

| Outcome | Default | Escape hatch |
|---|---|---|
| Proven | silent (listed in `plat prove --json` report) | none needed |
| Refuted | **error** `PT3001` with counterexample | fix the code or the contract |
| Unknown | **error** `PT3002` | `#[runtime_check]` on the fn, or `--unproven=runtime` |

- `#[runtime_check]` on a function turns its unprovable obligations into runtime checks that panic with the contract's source text and the offending values. Refuted obligations are still errors.
- `plat build --unproven=runtime|error` sets the project default. `plat.toml` can set it per module.
- `plat check` does types and effects only (fast, no Z3). `plat prove` adds verification. `plat build` and `plat test` run both.
- **Code with no refinements and no contracts produces no obligations** (other than indexing and fixed-width overflow, which can be switched to runtime with one setting). This is the gradual part.

## Counterexamples

When a VC is refuted, the diagnostic includes concrete values for the relevant variables, rendered in Platinum syntax:

Using `examples/broken/bank_fee.plat` (a withdrawal that charges a fee but forgets the fee in its balance check):

```text
error[PT3001]: refinement may not hold
  --> examples/broken/bank_fee.plat:10:25
   |
 9 |   if amt > acct.balance { return Err(Insufficient) }
   |      ------------------ this condition was false
10 |   Ok(Account { ..acct, balance: acct.balance - amt - FEE })
   |                        ^^^^^^^ field `balance` requires `self >= 0` (from type Account)
   |
   = counterexample:
       acct = Account { id: 0, balance: 5, open?: true }
       amt  = 5
       FEE  = 1
     so balance would be 5 - 5 - 1 = -1
   = help: the check on line 9 does not include `FEE`; did you mean `amt + FEE > acct.balance`?
```

The help line comes from a simple heuristic in v1: when a refuted obligation mentions a term (`FEE`) that no path condition mentions, suggest adding it to the nearest guarding comparison. Heuristics are fixes with applicability `maybe-incorrect`.

In JSON, `counterexample` is a structured object (see `09-tooling.md`) so an agent can turn it directly into a regression test. `plat fix --plan` can offer "add failing test from counterexample" as a fix.

## Boundaries: getting into a refined type

Unrefined data (user input, parsed JSON) enters refined types through checks, which the path-sensitive analysis understands:

```platinum
fn parse_amount(s: String) -> Option[Cents] {
  let n = Int.parse(s)?          // Option propagation
  if n >= 0 { Some(n) } else { None }   // inside the branch, n: Int where n >= 0, so n: Cents
}
```

Every refinement alias also gets a generated smart constructor: `Cents.check(n) -> Option[Cents]`.

## Purity requirement

Predicates may only call pure functions (no `needs`, declared or inferred), so that verification never depends on the outside world (`PT3010`).

## What v1 deliberately does not do

- Termination proofs (`decreases` is reserved).
- Invariant inference (Liquid-style inference via Horn clauses is a good later milestone).
- Heap reasoning (unnecessary: Platinum has value semantics, see `07-memory.md`). This is a big reason refinement checking stays tractable.
