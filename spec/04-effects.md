# 04. Effects as capabilities

## The idea

An **effect** is anything a function does besides compute its return value: read a file, call the network, read the clock, generate randomness, print. In Platinum, effects are **capabilities**: named interfaces a function needs from its caller. A function's signature lists them with `needs`.

```platinum
pub fn charge(card: Card, amt: Money) -> Result[Receipt, Declined | NetworkDown]
  needs Gateway, Clock
{ ... }
```

Reading the signature tells you, without reading the body, that `charge` can use the payment gateway and the clock, and nothing else: no filesystem, no arbitrary network, no randomness.

This is the "capability" reading of effects popularized by the Effekt language: the effect type says what the computation **requires from its context**, rather than what it might do to the world.

## Built-in capabilities

| Capability | Provides |
|---|---|
| `Console` | `puts`, `print`, `read_line` |
| `Fs` | file read/write/list |
| `Net` | raw TCP/HTTP client |
| `Clock` | `now`, `sleep` |
| `Random` | random numbers |
| `Env` | environment variables, process args |
| `Async` | `scope`, `spawn` (see 08-concurrency.md) |
| `Panic` | implicit everywhere; not tracked (see 06-errors.md) |

## User-defined capabilities

```platinum
cap Gateway {
  fn refund(id: PaymentId, amt: Money) -> Result[Refund, GatewayError]
  fn status(id: PaymentId) -> PaymentStatus
}
```

Operations are called through the capability name: `Gateway.refund(id, amt)`. Calling an operation requires `Gateway` to be in the current function's capability set.

A capability operation may itself need other capabilities when implemented, but that is the implementation's concern, not the caller's.

## Providing capabilities: handlers

A **handler** is any value implementing the capability's operations. Capability operations are declared without `self`; in an `impl Cap for Type`, each operation gains a leading `self` parameter (the handler value, which is how a fake can carry configuration). `handle Cap with value { body }` makes `value` provide `Cap` inside `body`; `body` no longer needs `Cap` from outside.

```platinum
type FakeGateway = { refunds: List[Money] }

impl Gateway for FakeGateway {
  fn refund(self, id: PaymentId, amt: Money) -> Result[Refund, GatewayError] {
    Ok(Refund { id, amount: amt })
  }
  fn status(self, id: PaymentId) -> PaymentStatus { Settled }
}

test "refund within limit succeeds" {
  handle Gateway with FakeGateway { refunds: [] } {
    let p = sample_payment(cents: 5000)
    assert refund(p, Money { cents: 1000, ccy: USD }).ok?
  }
}
```

Real implementations are provided by the runtime for built-ins (`main` receives them) or by libraries for user capabilities:

```platinum
fn main() needs Console, Net, Env {
  let key = Env.get("STRIPE_KEY").expect("missing key")
  handle Gateway with StripeGateway.new(key) {   // StripeGateway's impl needs Net
    run_app()
  }
}
```

## Rules

1. **Declared at the boundary.** Every `pub fn` must list its capabilities with `needs`. Missing ones: error `PT2001` with a fix plan adding the exact `needs` clause.
2. **Inferred inside.** Non-`pub` functions may omit `needs`; the compiler infers their capability set from the call graph (a fixpoint over the module's call graph). `plat check --json` reports inferred sets so agents can see them.
3. **Subset rule.** A function may call another only if the callee's capabilities are a subset of the caller's available set (its own `needs` plus any enclosing `handle`).
4. **`main`** may need any built-in capabilities; the runtime provides them. User capabilities must be provided with `handle` before use.
5. **Pure by default.** A function with no `needs` (declared or inferred) is pure. Contracts (`requires`/`ensures`/`where`) may only call pure functions (`PT3010`).

## Blocks vs closures (the effect polymorphism story)

Effect systems usually need "effect polymorphism" so that `map` can accept both pure and effectful functions. Platinum solves this the way Effekt does, and it lines up neatly with Ruby:

- **Trailing blocks** (`xs.each { |x| ... }`) are **second-class**: they can be called by the function they are passed to, but cannot be stored, returned, or captured by another closure. Because a block can never escape, it runs while its creator is still on the stack, so **the block's capabilities are simply the caller's capabilities**. `map`, `each`, `fold`, `retry`, `with_lock` all work with effectful blocks with no annotations.
- **First-class closures** (`|x| x + 1`, `fn(x: Int) -> Int { ... }`) are values. Their type carries their capabilities: `fn(Int) -> Int needs Net`. Storing an effectful closure requires spelling that out.

Declaring a block parameter:

```platinum
fn retry[T, E](times: Int where self > 0, body: block() -> Result[T, E]) -> Result[T, E] {
  var n = 1
  while true {
    match body() {
      Ok(v) => return Ok(v),
      Err(e) if n < times => { n += 1 },
      Err(e) => return Err(e),
    }
  }
}

let user = retry(3) { Http.get_json("/me") }?   // block uses Net; caller must have Net
```

`block(A, B) -> C` is the type of a second-class block parameter. It is only legal as the type of a function parameter (`PT2010` elsewhere).

## Semantics and implementation

- **v1 effects are tail-resumptive:** an operation runs and returns a value to its caller. There are no resumable continuations, so no need for CPS or stack switching. (Full algebraic effects with `resume` are a possible v2 extension; see DECISIONS D-07.)
- **Implementation: implicit capability passing.** After checking, each function that needs capabilities receives a hidden parameter: a record of capability dictionaries (vtables). `handle C with v { body }` extends that record for `body`. This compiles to ordinary function calls, fast in every backend.
- **Supply-chain property:** because a dependency's public functions must declare `needs`, `plat deps audit` can list exactly which capabilities each dependency uses, and flag changes between versions.
