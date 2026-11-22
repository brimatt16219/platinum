# 06. Errors

## Principles

- **Recoverable errors are values:** `Result[T, E]` and `Option[T]`.
- **No exceptions, no null.**
- **Bugs panic:** `panic`, failed `#[runtime_check]` contracts, and failed `unwrap`/`expect` abort the current task scope (see `08-concurrency.md`). Panics are not catchable in user code except at a `scope` boundary in tests (`assert_panics { ... }`).

## Result and Option

```platinum
type Option[T] = Some(T) | None
type Result[T, E] = Ok(T) | Err(E)
```

Common methods: `ok?`, `err?`, `some?`, `none?`, `map`, `map_err`, `and_then`, `unwrap_or`, `unwrap` (panics), `expect(msg)` (panics with message), `ok` (Result to Option).

Under verification, `x.unwrap` creates an obligation `x.ok?` (or `x.some?`). So `unwrap` is fine where the compiler can prove it safe.

## The `?` operator

`expr?` evaluates `expr`:
- on `Ok(v)` / `Some(v)` yields `v`,
- on `Err(e)` returns `Err(e)` from the enclosing function (after widening `e`, below),
- on `None` returns `None` from the enclosing function.

`?` on `Result` requires the enclosing function to return `Result[_, F]` where the error type of `expr` is a **subtype** of `F`. `?` on `Option` requires the enclosing function to return `Option[_]`. Mixing is an error (`PT1301`) with a fix plan (`.ok_or(Tag)` or `.ok`).

## Tag unions compose without wrapper enums

```platinum
fn find_user(id: UserId) -> Result[User, NotFound]
fn fetch_orders(u: User) -> Result[List[Order], Timeout(after: Int) | NotFound]

fn dashboard(id: UserId) -> Result[Dashboard, NotFound | Timeout(after: Int)] {
  let u = find_user(id)?          // NotFound widens into the union
  let os = fetch_orders(u)?
  Ok(Dashboard { user: u, orders: os })
}

match dashboard(id) {
  Ok(d) => render(d),
  Err(NotFound) => not_found_page(),
  Err(Timeout(after)) => retry_later(after),
}
```

Tag union rules:
- A tag is an uppercase name, optionally with a payload: `NotFound`, `Timeout(after: Int)`.
- The same tag name with different payload types in one union is an error (`PT1310`).
- `A | B` and `B | A` are the same type; duplicates collapse.
- `A <: A | B` (width subtyping). This is the only subtyping in the type system besides refinements.
- Declared variants (`type Currency = USD | EUR`) are **nominal** and do not mix with tags. To use a declared error enum, put it in the union as a single member: `Result[T, DbError | NotFound]`.

## Panics

`panic("message")` has type `Never` (the empty type, subtype of everything). `Never` is also the type of `return`, `break`, `continue`. Panics in `main` print the message, the source location, and (for contract failures) the contract text and values, then exit with code 101.
