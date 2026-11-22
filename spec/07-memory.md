# 07. Memory and mutation

## Value semantics

All Platinum values behave as immutable values. There are no references, pointers, or shared mutable objects in the surface language. Passing a value to a function can never let that function change what the caller sees.

This makes reasoning simple for humans, agents, and the verifier (no aliasing, no heap reasoning) and is what makes reference counting cycle-free.

## Local mutation: `var`

```platinum
var total = 0
for i in invoices { total += i.amount.cents }
```

- `var` bindings can be reassigned. They are local: closures capture a `var` **by value** at creation time (lint `PT1501` warns if a closure captures a `var` that is later modified). Trailing blocks, being second-class, can read and assign the caller's `var`s directly, which is what makes `each { |x| total += x }` work.
- Fields of a `var` record can be assigned: `acct.balance -= 10` means `acct = Account { ..acct, balance: acct.balance - 10 }`.
- `xs[i] = v` on a `var` list means `xs = xs.set(i, v)`.

## Bang methods (`!`)

By convention a method `name!` is the in-place variant of `name`. For any method `name` with signature `fn name(self, ...) -> Self`, the compiler auto-derives `name!` usable only on a `var` place:

```platinum
var xs = [3, 1, 2]
xs.sort!          // means xs = xs.sort
xs.push!(4)       // means xs = xs.push(4)
```

Calling `name!` on a `let` binding is an error (`PT1502`) with a fix plan changing `let` to `var`.

## Runtime representation: Perceus reference counting

Platinum uses **Perceus** (Reinking, Xie, de Moura, Leijen; PLDI 2021), as in Koka, Lean 4, and Roc:

1. Heap values (records, variants with payloads, lists, strings, maps, closures) carry a reference count.
2. The compiler inserts **precise** `dup` (increment) and `drop` (decrement) instructions, based on ownership in a linear resource calculus: a value is dropped right after its last use, not at scope end. Programs are "garbage free".
3. **Reuse analysis:** when a value being consumed is uniquely owned (refcount 1) and a new value of the same size is constructed, the memory is reused in place. Combined with the `!` desugaring above, `xs.push!(4)` on a uniquely owned list mutates in place with no copy. Koka calls this style "functional but in place" (FBIP).
4. Because the language has no mutable references, reference cycles cannot be created, so there is no cycle collector.

### By implementation stage

| Stage | Strategy |
|---|---|
| M1 to M5 interpreter | Rust `Rc<Value>` with `Rc::make_mut` for copy-on-write. This already behaves like reuse: `make_mut` mutates in place when the count is 1 and clones otherwise. Good for learning; measure it. |
| M6 VM | Explicit Perceus pass on a lowered IR: insert `dup`/`drop`, then drop-guided reuse. |
| M8 native | Same IR and pass, emitted through Cranelift with inline refcount ops. |

## Future (not v1)

- A restricted `Ref[T]` cell for rare shared mutable state, behind a capability (`needs Mut`) so it stays visible in signatures.
- Borrowing parameters (`borrow x: T`) as an optimization to elide `dup`/`drop` pairs; Koka and Lean infer most of these.
