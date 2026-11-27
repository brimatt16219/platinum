# Platinum

> A small, functional-first language where the compiler proves what your code promises, built for humans and AI agents to write together.

Platinum (symbol: **Pt**) combines three ideas that no single language ships together today:

1. **Refinement types and contracts proven at compile time.** `Int where self > 0`, `requires`, `ensures`, checked by an SMT solver (Z3). When a proof fails, you get a concrete counterexample.
2. **Effects as capabilities.** Every public function says what it can touch (`needs Net, Clock`). Tests swap in fakes with `handle`. A dependency cannot phone home without saying so in its signature.
3. **Agent-native tooling.** Every command speaks JSON, every error has a stable code and a machine-applicable fix plan, and `plat mcp` exposes the compiler to agents over MCP.

**Why "Platinum"?** Rust is a metal, Ruby is a precious material, and platinum is a precious metal. Platinum takes Rust's strictness (braces, `Result` and `?`, traits, a compiler that refuses bad code) and Ruby's joy (trailing blocks, `empty?` predicates, `sort!`, `#{interpolation}`, chain everything), on a core that both humans and agents write reliably.

```platinum
type Money = { cents: Int where self >= 0, ccy: Currency }

pub fn refund(p: Payment, amt: Money) -> Result[Refund, OverLimit | Closed]
  needs Gateway
  requires amt.ccy == p.amount.ccy
  ensures result.ok? => result.unwrap.amount.cents <= p.amount.cents
{
  if p.closed? { return Err(Closed) }
  if amt.cents > p.amount.cents { return Err(OverLimit) }
  let r = Gateway.refund(p.id, amt)?
  Ok(r)
}
```

## Repository map

| File | What it is |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Instructions for Claude Code: how to build the compiler, in what order, with what crates |
| [ROADMAP.md](ROADMAP.md) | Staged milestones M0 to M9, each with acceptance criteria and learning goals |
| [DECISIONS.md](DECISIONS.md) | Every design decision, the alternatives considered, and why |
| [LEARNING.md](LEARNING.md) | Concepts and reading list, mapped to milestones |
| [spec/00-overview.md](spec/00-overview.md) | Vision, pillars, non-goals, competitive landscape |
| [spec/01-lexical.md](spec/01-lexical.md) | Tokens, identifiers, literals, comments |
| [spec/02-grammar.md](spec/02-grammar.md) | EBNF grammar and precedence table |
| [spec/03-types.md](spec/03-types.md) | Type system, inference, traits, numbers, records, variants |
| [spec/04-effects.md](spec/04-effects.md) | Capabilities, handlers, blocks vs closures |
| [spec/05-verification.md](spec/05-verification.md) | Refinements, contracts, SMT encoding, counterexamples |
| [spec/06-errors.md](spec/06-errors.md) | Result, `?`, tag unions, panics |
| [spec/07-memory.md](spec/07-memory.md) | Value semantics, `var`, `!` methods, Perceus |
| [spec/08-concurrency.md](spec/08-concurrency.md) | Structured concurrency via the `Async` capability |
| [spec/09-tooling.md](spec/09-tooling.md) | CLI, JSON diagnostics, MCP server, LSP, content addressing |
| [spec/10-stdlib.md](spec/10-stdlib.md) | Prelude and minimal standard library |
| [examples/](examples/) | Sample programs used as golden tests |

## Status

Design spec v0.1 (November 2026). **M0 (foundations) is complete:** the Cargo workspace, diagnostics with JSON and human rendering, the full lexer, and `plat check` / `plat explain`. Next is M1 (parser and tree-walking interpreter); see `ROADMAP.md` and `notes/M0-retro.md`.

```powershell
cargo test
```

```powershell
cargo run -q -p plat-cli -- check examples/refund.plat
```

```powershell
cargo run -q -p plat-cli -- explain PT0007
```
