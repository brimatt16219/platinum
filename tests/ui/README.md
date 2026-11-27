# Golden tests

Each `.plat` file here is a test case. The harness (`crates/plat-cli/tests/ui.rs`) runs

```text
plat check --json <file>
```

from this directory and compares the output with the file next to it: `foo.plat` is checked against `foo.diag.json`. The exit code must agree too (1 if there are errors, 0 otherwise). Later milestones add `foo.stdout` for expected program output.

## Layout

| Folder | What it covers | Since |
|---|---|---|
| `lex/` | lexical errors (PT0001 to PT0099) and valid token forms | M0 |

Start each `.plat` file with a comment saying what it tests and what the expected outcome is.

## Adding a test

1. Write `some_case.plat`.
2. Generate its expected output:

   ```powershell
   $env:PLAT_BLESS=1; cargo test -p plat-cli --test ui; Remove-Item Env:PLAT_BLESS
   ```

3. Read `some_case.diag.json` and confirm it is what a user should see. The expected files *are* the specification of the compiler's output, so never bless without reading the diff.

## Rules the harness enforces

- Every `.plat` has a matching `.diag.json`.
- Every `.diag.json` has a matching `.plat` (no leftovers after a rename).
- Every lexer code in the registry (`crates/plat-diag/src/codes.rs`) is produced by at least one test here.
- Files use LF line endings (`.gitattributes`), because diagnostics contain byte offsets.
