# 09. Tooling

Tooling is part of the language, not an afterthought. Two audiences: humans (pretty output, LSP) and agents (JSON, MCP). Both read from the same data.

## CLI: `plat`

| Command | Does | Milestone |
|---|---|---|
| `plat run <file>` | check (types + effects) then run in the interpreter (later: VM) | M1 |
| `plat check [paths]` | lex, parse, resolve, type, effect check; no Z3 | M2 |
| `plat prove [paths]` | `check` + verification | M4 |
| `plat test [paths] [--seed N]` | run `test` blocks | M3 |
| `plat fmt [--check]` | canonical formatter (one style, no options) | M1 (basic) |
| `plat explain <CODE>` | long explanation of a diagnostic code with examples | M0 |
| `plat fix [--plan] [--apply]` | list or apply machine fixes from the last check | M2 |
| `plat repl` | interactive loop | M1 |
| `plat mcp` | run the MCP server over stdio | M5 |
| `plat lsp` | run the language server | M5 |
| `plat build` | produce bytecode (M6) or native binary (M8) | M6 |
| `plat hash <def>` / `plat diff --semantic` | definition hashes, semantic diff | M9 |

**Every** command accepts `--json`. In JSON mode, stdout carries exactly one JSON document (or JSON Lines for streaming commands like `test`), and nothing else. Exit codes: `0` success, `1` diagnostics with errors, `2` usage error, `101` program panic.

## Diagnostic codes

| Range | Area |
|---|---|
| `PT0001`-`PT0099` | lexing |
| `PT0100`-`PT0999` | parsing |
| `PT1000`-`PT1999` | names, types, traits, patterns, numbers, mutation |
| `PT2000`-`PT2999` | effects, capabilities, concurrency |
| `PT3000`-`PT3999` | verification |
| `PT4000`-`PT4999` | runtime (panic reports) |
| `PT9000`-`PT9999` | internal compiler errors (always a bug) |

Codes are stable forever: never reused, only deprecated. Message wording may change; meaning is pinned by the code.

## JSON diagnostic schema (v1)

```json
{
  "schema": "platinum.diagnostics/v1",
  "tool": "plat prove",
  "summary": { "errors": 1, "warnings": 0, "proven": 3, "refuted": 1, "unknown": 0 },
  "diagnostics": [
    {
      "code": "PT3001",
      "severity": "error",
      "message": "refinement may not hold",
      "primary": {
        "file": "examples/broken/bank_fee.plat",
        "start": { "line": 10, "col": 25, "offset": 412 },
        "end":   { "line": 10, "col": 32, "offset": 419 },
        "label": "field `balance` requires `self >= 0` (from type Account)"
      },
      "secondary": [
        { "file": "examples/broken/bank_fee.plat",
          "start": { "line": 9, "col": 6, "offset": 360 },
          "end": { "line": 9, "col": 24, "offset": 378 },
          "label": "this condition was false" }
      ],
      "notes": ["balance would be 5 - 5 - 1 = -1"],
      "counterexample": {
        "bindings": [
          { "name": "acct", "type": "Account", "value": "Account { id: 0, balance: 5, open?: true }" },
          { "name": "amt", "type": "Int", "value": "5" },
          { "name": "FEE", "type": "Int", "value": "1" }
        ],
        "path": [ { "line": 9, "condition": "amt > acct.balance", "taken": false } ]
      },
      "fixes": [
        {
          "id": "fix-1",
          "title": "include FEE in the balance check",
          "applicability": "maybe-incorrect",
          "edits": [ { "file": "examples/broken/bank_fee.plat", "start": 360, "end": 363, "replacement": "amt + FEE" } ]
        },
        {
          "id": "fix-2",
          "title": "add regression test from counterexample",
          "applicability": "machine-applicable",
          "edits": [ { "file": "examples/broken/bank_fee_test.plat", "start": 0, "end": 0,
                       "replacement": "test \"withdraw_with_fee PT3001 counterexample\" { ... }\n" } ]
        }
      ],
      "docs": "plat explain PT3001"
    }
  ],
  "inferred": {
    "capabilities": [ { "fn": "bank_fee.helper", "needs": [] } ]
  }
}
```

Positions: `line` and `col` are 1-based, `col` counts Unicode scalar values (characters), and `offset` is a 0-based UTF-8 byte offset. Fix `edits` use byte offsets. (DECISIONS D-19.)

`applicability` values (borrowed from rustc): `machine-applicable` (safe to apply automatically), `maybe-incorrect` (plausible, needs review), `has-placeholders` (contains `<...>` the agent or human must fill).

Implementation notes (M0):

- Human output goes to stderr; in JSON mode stdout carries the one document and stderr stays empty.
- `summary` currently carries `errors` and `warnings`; `proven`, `refuted`, and `unknown` are added when `plat prove` exists (M4). The `inferred` object arrives with effect inference (M3). Consumers must ignore fields they do not know.
- Problems that leave no source to point at (unreadable file, unknown diagnostic code, bad arguments) are usage errors: a message on stderr and exit code `2`, not a diagnostic.

## `plat explain --json`

```json
{
  "schema": "platinum.explain/v1",
  "code": "PT0005",
  "title": "malformed number literal",
  "area": "lexing",
  "explanation": "A number literal is not well formed. ..."
}
```

`code` is always in canonical form (`pt0005` on the command line is accepted and normalized). `area` is the range name from the code table above (`null` for a code outside every range). The MCP `explain` tool returns the same object.

## MCP server (`plat mcp`)

Exposes the compiler directly to agents over the Model Context Protocol (stdio transport). Tools:

| Tool | Input | Output |
|---|---|---|
| `check` | `{ paths: [..] }` or `{ source: "..." }` | diagnostics JSON (schema above) |
| `prove` | same, plus optional `fn` filter | diagnostics JSON with proof summary per obligation |
| `run` | `{ path, args?, stdin?, capabilities?: {...} }` | stdout, stderr, exit code; capabilities can be restricted or faked |
| `test` | `{ paths?, filter?, seed? }` | per-test results |
| `type_at` | `{ path, line, col }` | type, refinement, and capabilities of the expression |
| `signature` | `{ name }` | full signature incl. `needs`, `requires`, `ensures` |
| `effects_of` | `{ name }` | inferred capability set and the call path that requires each one |
| `explain` | `{ code }` | long-form explanation |
| `fix_plan` | `{ diagnostic_id }` | fixes with edits |
| `apply_fix` | `{ fix_id }` | updated files list (requires confirmation flag) |
| `fmt` | `{ source }` | formatted source |

Resources: `platinum://spec/{section}` serves this spec, `platinum://std/{module}` serves stdlib docs. This lets an agent with no prior knowledge of Platinum learn it in-session.

## LSP (`plat lsp`)

Diagnostics on save (types and effects instantly; proofs in the background with a status indicator), hover (type + refinement + capabilities), go to definition, code actions from `fixes`, inlay hints for inferred `needs` on private functions.

## Content-addressed definitions

From M0, every top-level definition gets a **definition hash**:

1. Lower to HIR, resolve names.
2. Normalize: strip spans, comments, and formatting; alpha-rename locals to positional indices; replace references to other definitions with **their** hashes.
3. Serialize canonically and hash with BLAKE3.

Uses by stage:
- M2+: incremental check cache keyed by hash (unchanged definition and dependencies: reuse results, including proofs, which are the expensive part).
- M5: MCP `signature` and LSP use hashes to detect real changes.
- M9: `plat diff --semantic` (a rename or reformat shows as no change), proof certificates stored by hash, and dependency resolution where two versions of a library can coexist because definitions are identified by hash (Unison's idea), without moving to a database-only codebase.
