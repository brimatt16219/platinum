# 01. Lexical structure

Source files are UTF-8 with the extension `.plat`. Newlines are not significant except inside string literals; statements are separated by newlines **or** semicolons (see "Statement termination").

## Comments

```platinum
// line comment
/// doc comment, attaches to the next item, rendered by `plat doc`
/* block comment /* nesting allowed */ */
```

## Identifiers

| Kind | Pattern | Used for |
|---|---|---|
| Lower identifier | `[a-z_][A-Za-z0-9_]*` optionally followed by exactly one `?` or `!` | values, functions, fields, modules |
| Upper identifier | `[A-Z][A-Za-z0-9_]*` | types, constructors, tags, traits, capabilities |

### The `?` / `!` suffix rule (Ruby flavor)

- A lower identifier immediately followed (no whitespace) by `?` absorbs it: `empty?`, `paid?`. By convention, `?`-names return `Bool`. The compiler enforces this for declarations: a function or field whose name ends in `?` must have type `Bool` (diagnostic `PT1040`).
- A lower identifier immediately followed by `!` absorbs it **unless** the next character is `=` (so `a!=b` still lexes as `a != b`). By convention, `!`-names are mutating variants (see `spec/07-memory.md`).
- **Error propagation `?`** is postfix and applies to any expression. After `)`, `]`, `}`, a literal, or whitespace, `?` is always the propagation operator. After a bare identifier with no whitespace, the lexer produces the identifier `name?`; the name resolver then applies the **fallback rule**: if `name?` is not bound but `name` is bound and has type `Result[..]` or `Option[..]`, the expression is rewritten to `(name)?`. The same rule applies after a dot: in `task.await?` or `resp.body?`, if no field or method named `await?` exists but `await` does and yields a `Result` or `Option`, it is read as `(task.await)?`. The formatter leaves it as written, and lint `PT1041` (warning, off by default) suggests `(name)?` for clarity.

## Keywords

```
as  assert  block  break  cap  const  continue  else  ensures  false  fn  for  handle
if  impl  import  in  invariant  let  match  needs  opaque  pub  requires
result  return  scope  self  spawn  test  trait  true  type  var  where  while  with
```

`result` and `self` are contextual: `result` is only a keyword inside `ensures`; `self` refers to the receiver in methods and to the refined value inside a `where` predicate.

Reserved for future use: `async`, `decreases`, `macro`, `unsafe`, `yield`.

## Literals

| Literal | Examples | Type |
|---|---|---|
| Integer | `0`, `42`, `1_000_000`, `0xFF`, `0b1010` | `Int` (arbitrary precision) |
| Sized integer | `42i64`, `255u8`, `7i32` | `I64`, `U8`, `I32` |
| Decimal | `1.50`, `0.1`, `1_000.00` | `Decimal` (exact, base 10) |
| Float | `1.5f`, `2.0e-3f`, `6.02e23f` | `Float` (IEEE 754 f64) |
| String | `"hello #{name}"` | `String` (interpolated) |
| Raw string | `r"C:\no\#{escapes}"`, `r#"has "quotes""#` | `String` |
| Multiline string | `"""` ... `"""` (leading indentation stripped) | `String` |
| Bool | `true`, `false` | `Bool` |
| Unit | `()` | `Unit` |

**Important:** a number with a decimal point and no `f` suffix is a `Decimal`. This is deliberate: money and other human quantities should never be floats by accident.

Number literal rules (DECISIONS D-22):

- `_` separates digits and must sit between two digits: `1_000` is valid; `1_`, `1__0`, `0x_FF`, `1.5_` are errors (`PT0005`).
- An exponent (`e3`, `e-3`, `E+3`) is only allowed on a `Float`, so it requires the `f` suffix: `2.0e-3f`, `1e5f`. `1e5` is an error (`PT0005`); write Decimals out in full.
- Digits followed directly by `f` are a `Float` even without a decimal point: `1f`.
- Size suffixes (`i8` to `u64`) apply to whole numbers in any base (`42i64`, `0xFFu8`, `0b1i8`), not to decimals (`1.5i32` is `PT0006`). `f` applies only to base-10 numbers; in hex, `f` is a digit.
- Base prefixes are lowercase only (`0x`, `0b`); hex digits may be either case. There are no octal literals. Leading zeros are allowed (`007` is 7).
- A `.` starts a fractional part only when a digit follows it, so `1..5` and `1.max(2)` keep their `.`. Directly after a `.` token no fraction is read, so tuple access `pair.0.1` means `(pair.0).1`.

String escapes: `\n \t \r \\ \" \# \u{1F600}`. Interpolation `#{expr}` requires `expr` to implement `Show`.

String literal rules (DECISIONS D-23):

- Only triple-quoted strings span lines. An ordinary `"..."` or raw `r"..."` string that reaches the end of its line is an error (`PT0002`), reported at the opening quote.
- A `#{` inside an ordinary string must be closed on the same line (`PT0007`). Inside a triple-quoted string an interpolation may span lines.
- A triple-quoted string ends at the first `"""`. Inside it, `"` and `""` are ordinary text; a `"` directly before the closing `"""` must be escaped as `\"`.
- Raw strings are `r`, zero or more `#`, then `"`, and end at `"` followed by the same number of `#`: `r#"say "hi""#`. They have no escapes and no interpolation. There is no raw triple-quoted form.
- `\u{...}` takes 1 to 6 hex digits naming a Unicode scalar value (no surrogates `D800` to `DFFF`, nothing above `10FFFF`); anything else is `PT0004`.

## Operators and punctuation

```
+  -  *  /  %  ==  !=  <  <=  >  >=  &&  ||  !
=  +=  -=  *=  /=
->  =>  |>  ?  .  ..  ..=  :  ,  ;  |
(  )  [  ]  {  }  #[
```

- `|>` pipe: `x |> f(a)` means `f(x, a)`.
- `..` exclusive range, `..=` inclusive range.
- `|` separates variants, union members, and block parameters (`{ |x| ... }`).

## Statement termination

A newline ends a statement when the line could syntactically end there and the next line does not begin with a binary operator, `.`, `|>`, `?`, `|`, or `->`. This permits Ruby-style leading-dot chains, multi-line variant declarations (`| Circle(...)` on each line), and return types on their own line:

```platinum
let total = invoices
  .reject { |i| i.paid? }
  .map { |i| i.amount }
  .sum
```

Semicolons may separate statements on one line.

Implementation (DECISIONS D-17): the lexer owns this rule. It emits a `Newline` token only when the previous token can end a statement (an identifier, literal, `)`, `]`, `}`, `?`, or one of `return break continue self result true false`) and the next non-blank token is not a binary operator, `.`, `|>`, `?`, `|`, or `->` (`|` and `->` added by D-21). Newlines inside `( )` and `[ ]` are never significant, so argument lists and list literals may span lines freely.

The parser additionally ignores `Newline` tokens inside item headers (DECISIONS D-21): between `fn` and the opening `{` of its body (so `needs`, `requires`, `ensures`, and the `{` itself may each start a line), in `while`/`for` heads before the loop body (so `invariant` clauses may each start a line), and around `=` in `type` declarations. A statement cannot end in those positions, so this never changes meaning. `{` is deliberately *not* a lexer continuation token: `foo()` followed by `{ ... }` on the next line stays two statements rather than becoming a trailing-block call.

String interpolation (DECISIONS D-18) is lexed as a token stream: `"a #{x} b"` becomes `StrStart("a ")`, `InterpStart`, the tokens of `x`, `InterpEnd`, `StrEnd(" b")`.
