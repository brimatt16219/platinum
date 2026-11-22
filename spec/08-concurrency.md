# 08. Concurrency

## Model: structured concurrency through the `Async` capability

Concurrency is just another capability. A function that starts concurrent work `needs Async`. There is no `async fn` / sync fn split ("function coloring"): any function can call any other, subject to the ordinary capability rules.

```platinum
fn load_dashboard(id: UserId) -> Result[Dashboard, NotFound | Timeout(after: Int)]
  needs Async, Db, Http
{
  scope {
    let user = spawn Db.find_user(id)
    let feed = spawn Http.get_feed(id)
    Ok(Dashboard { user: user.await?, feed: feed.await? })
  }
}
```

## Rules

1. `scope { body }` opens a **task group**. `spawn expr` starts `expr` as a child task in the innermost enclosing `scope` and immediately returns a `Task[T]`.
2. `spawn` outside a `scope` is a compile error (`PT2101`). Tasks cannot be returned from or stored outside their scope (`Task[T]` is second-class, like blocks).
3. **Leaving a scope waits for every child task.** No task outlives the block that started it. This is the core of structured concurrency (the "nursery" idea from Python's Trio, also adopted by Kotlin and Swift).
4. `task.await` returns the task's value. Awaiting is optional; un-awaited tasks are still joined at scope exit, and their values are discarded (lint `PT2102` if the value is not `Unit`).
5. **Failure:** if a child task panics, the scope cancels all sibling tasks and re-raises the panic at the scope boundary. Errors returned as `Err` values are ordinary values; it is the code awaiting them that decides (with `?`) to bail out, which then cancels the remaining siblings when the scope exits early.
6. **Cancellation** is cooperative: capability operations (`Clock.sleep`, `Net`, `Fs`, `Async` operations) are cancellation points.
7. Capabilities flow into spawned tasks: a spawned expression may use exactly the capabilities available at the `spawn` site.
8. Data sharing is safe by construction: values are immutable, and `var` locals cannot be captured mutably by `spawn` (`PT2103`). Communication between tasks uses `Channel[T]` from the standard library (operations need `Async`).

## Timeouts

```platinum
let r = Async.timeout(ms: 500) { Http.get_feed(id) }   // Result[Feed, TimedOut]
```

## Implementation by stage

| Stage | Runtime |
|---|---|
| Interpreter (M7) | Single-threaded cooperative scheduler with a deterministic, seedable task order. Determinism makes tests reproducible: `plat test --seed 42`. |
| VM | Green threads (work-stealing pool) once Perceus refcounts are made atomic for values that cross threads (or task-local heaps with copy on send). Decide in a DECISIONS entry at M6. |
| Native | Same as VM. |
