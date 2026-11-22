# 03. Types

## Overview

Platinum is statically and strongly typed. Type checking is **bidirectional** (checking mode pushes known types inward, synthesis mode infers types outward) with **Hindley-Milner style unification** for local inference and generics.

**Annotation rule:** top-level `fn` signatures must annotate all parameter types and the return type (return type may be omitted only when it is `Unit`). Everything inside a function body is inferred. Lambdas are inferred from context; annotate their parameters when there is no context.

Why: signatures are documentation for both humans and agents, give much better error locality, and are where refinement checking needs known types.

## Built-in types

| Type | Description |
|---|---|
| `Bool` | `true` / `false` |
| `Int` | arbitrary-precision integer; never overflows |
| `I8 I16 I32 I64 U8 U16 U32 U64` | fixed-width integers (opt-in, for performance and interop) |
| `Decimal` | exact base-10 decimal (money, measurements) |
| `Float` | IEEE 754 binary64 |
| `String` | immutable UTF-8 text |
| `Unit` | the type of `()` |
| `List[T]` | immutable persistent list/vector (indexable, see stdlib) |
| `Map[K, V]`, `Set[T]` | immutable hash map / set (`K: Hash + Eq`) |
| `Option[T]` | `Some(T) \| None` |
| `Result[T, E]` | `Ok(T) \| Err(E)` |
| `(A, B, ...)` | tuples |
| `fn(A, B) -> C needs Caps` | first-class function values |

### Numeric rules

- No implicit conversions between numeric types. Use `x.to_decimal`, `x.to_float`, `I64.try_from(n)` (returns `Option`), `I64.wrapping_from(n)`.
- Fixed-width arithmetic must not overflow. Under verification, each fixed-width operation generates an overflow obligation; if it cannot be proven, the program is rejected unless the function opts into runtime checks (see `05-verification.md`). Wrapping arithmetic is explicit: `a.wrapping_add(b)`.
- `Decimal` division requires a rounding mode when the result is inexact: `a.div(b, scale: 2, round: HalfEven)`. The `/` operator on `Decimal` is only allowed when the compiler can prove exactness or when the divisor is an integer literal power of ten; otherwise diagnostic `PT1210` with a fix plan inserting `.div(...)`.
- `Float` comparisons with `==` produce a warning (`PT1220`).
- `Int` and sized-int `/` truncate toward zero; `%` takes the sign of the dividend. Division and `%` by a value not provably non-zero create a verification obligation (`divisor != 0`); under `--unproven=runtime` it becomes a panic check. `Int.div_floor` and `Int.modulo` give floor semantics.

## User-defined types

### Records (nominal, product types)

```platinum
type Money = { cents: Int where self >= 0, ccy: Currency }

let m = Money { cents: 1250, ccy: USD }
let m2 = Money { ..m, cents: 0 }      // functional update
m.cents                                // field access
```

Two record types with identical fields are still different types.

### Variants (sum types / ADTs)

```platinum
type Currency = USD | EUR | GBP

type Shape =
  | Circle(radius: Float)
  | Rect(w: Float, h: Float)

fn area(s: Shape) -> Float {
  match s {
    Circle(r) => 3.14159f * r * r,
    Rect(w, h) => w * h,
  }
}
```

Matches must be **exhaustive** (`PT1101` lists the missing cases with a fix plan that inserts them). Unreachable arms warn (`PT1102`).

### Opaque types (newtypes)

```platinum
opaque type UserId = Int
opaque type OrderId = Int
```

Outside the defining module, `UserId` and `OrderId` are distinct and their representation is hidden. Inside the module, `UserId(n)` wraps and `id.raw` unwraps.

### Type aliases and refinement aliases

```platinum
type Cents = Int where self >= 0
type Percent = Decimal where self >= 0.00 && self <= 100.00
type NonEmpty[T] = List[T] where self.len > 0
```

An alias without `opaque` is transparent: `Cents` is `Int` plus a predicate.

### Refinement types (summary; full rules in 05-verification.md)

`T where P` is the subset of `T` for which predicate `P` holds; `self` names the value. Subtyping: `{T | P} <: {T | Q}` when `P => Q` is valid. The checker discharges these with Z3. Plain `T` is `T where true`.

## Generics

```platinum
fn first[T](xs: List[T]) -> Option[T] { ... }

fn largest[T: Ord](xs: NonEmpty[T]) -> T {
  xs.fold(xs[0]) { |best, x| if x > best { x } else { best } }
}
```

Generics are monomorphized in the VM and native backends; the interpreter uses uniform representation.

## Traits

```platinum
trait Show {
  fn show(self) -> String
}

impl Show for Money {
  fn show(self) -> String { "#{self.cents / 100}.#{self.cents % 100} #{self.ccy}" }
}
```

- Traits can have default method bodies.
- Coherence: an `impl Trait for Type` must live in the module defining `Trait` or `Type` (orphan rule).
- Built-in traits with derivation: `Eq`, `Ord`, `Hash`, `Show`, `Debug`, `Default`. Derive with an attribute: `#[derive(Eq, Hash, Show)] type Point = { x: Int, y: Int }`.
- Operator overloading through traits: `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Index`.
- No higher-kinded types in v1. `map`, `filter`, etc. are defined per collection.

## Tag unions (for errors)

Uppercase tags can be used without prior declaration inside a union type position, Roc style:

```platinum
fn load(id: UserId) -> Result[User, NotFound | Timeout(after: Int)]
```

Tag unions are **structural**: `NotFound | Timeout(after: Int)` written in two places is the same type, and a smaller union is a subtype of a larger one (`A <: A | B`). This is what makes `?` compose without wrapper enums (see `06-errors.md`). Tag unions are only permitted as the error parameter of `Result`, as the type of a `match` scrutinee, and in type aliases.

## Type inference notes for implementers

- Algorithm: bidirectional checking (Dunfield and Krishnaswami style) with unification variables and let-generalization only at top-level items (local `let` is not generalized; this keeps errors local and is what Rust, Gleam, and OCaml's value restriction approximate).
- Trait resolution: dictionary passing in the interpreter; monomorphization later.
- Refinements are erased for unification: the checker first infers base types, then the verifier (`plat-verify`) checks predicates as a separate pass over typed HIR. This separation keeps the type checker simple and makes `plat check` fast without Z3.
