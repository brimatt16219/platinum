# 10. Standard library (v1 minimum)

Keep it small; grow it with real programs. Modules live in `std/` and are written in Platinum where possible, with `#[builtin]` functions implemented in Rust.

## Prelude (auto-imported)

- Types: `Bool Int Decimal Float String Unit Never List Map Set Option Result Task`, sized ints.
- Constructors: `Some None Ok Err`.
- Traits: `Eq Ord Hash Show Debug Default Add Sub Mul Div Neg Index`.
- Functions: `panic`, `assert` (statement), `dbg` (pure, prints in debug builds only; allowed without `Console` like Rust's `dbg!`, stripped in release).
- Capabilities: `Console Fs Net Clock Random Env Async`.
- Console helpers (need `Console`): `puts`, `print`, `read_line`.

## Core modules

| Module | Highlights |
|---|---|
| `std.list` | `len empty? first last get(i) push pop concat map filter reject fold sum min max sort sort_by reverse each each_with_index zip take drop any? all? find count group_by uniq chunk flat_map` |
| `std.string` | `len empty? chars lines split join trim upcase downcase starts_with? ends_with? contains? replace parse_int` |
| `std.map` / `std.set` | `get insert remove has? keys values each map filter size` |
| `std.option` / `std.result` | combinators from `06-errors.md` |
| `std.int` / `std.decimal` / `std.float` | `abs pow min max clamp`, `Decimal.div(scale:, round:)`, rounding modes `HalfEven HalfUp Down Up` |
| `std.money` | `Money`, `Currency` (ISO 4217 codes), allocation without losing cents (`allocate(ratios)`), formatting. A showcase of refinements. |
| `std.json` | `parse(String) -> Result[Json, ParseError]`, `Json` variant type, `encode` via `Show`-like `ToJson` trait |
| `std.time` | `Instant`, `Duration`, `Date`; anything reading the clock needs `Clock` |
| `std.fs` | `read(path) write(path, s) list(dir)` (need `Fs`) |
| `std.http` | small client: `get post` returning `Result[Response, HttpError]` (need `Net`) |
| `std.async` | `Channel[T]`, `timeout`, `sleep` (need `Async`) |
| `std.test` | `assert_eq`, `assert_panics`, fakes: `FixedClock`, `SeededRandom`, `MemoryFs`, `RecordingConsole` |

The `std.test` fakes are the payoff of effects-as-capabilities: every built-in capability has an in-memory implementation, so tests never touch the real world unless they choose to.

## Contracts in the stdlib

Stdlib functions carry refinements and contracts, which is what lets user code prove things without annotations. Examples:

```platinum
fn get[T](xs: List[T], i: Int where self >= 0 && self < xs.len) -> T     // indexing is safe when provable
fn first[T](xs: List[T]) -> Option[T] ensures result.some? == (xs.len > 0)
fn push[T](xs: List[T], x: T) -> List[T] ensures result.len == xs.len + 1
fn abs(n: Int) -> Int ensures result >= 0
```
