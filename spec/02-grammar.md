# 02. Grammar

Notation: EBNF. `{ x }` zero or more, `[ x ]` optional, `( a | b )` choice, `"x"` terminal. `LIDENT` and `UIDENT` are lower and upper identifiers (see `01-lexical.md`). `NL` is a statement-terminating newline or `;`.

## Items

```ebnf
program       = { item } ;

item          = { attribute } ( import_decl
              | [ "pub" ] ( fn_decl | type_decl | trait_decl | impl_decl | cap_decl | const_decl )
              | test_decl ) ;

attribute     = "#[" LIDENT [ "(" attr_arg { "," attr_arg } ")" ] "]" ;   (* #[derive(Eq, Show)], #[runtime_check], #[measure] *)
attr_arg      = LIDENT | UIDENT | literal ;

import_decl   = "import" module_path [ "as" LIDENT ] [ "{" import_list "}" ] NL ;
module_path   = LIDENT { "." LIDENT } ;
import_list   = ( LIDENT | UIDENT ) { "," ( LIDENT | UIDENT ) } [ "," ] ;

const_decl    = "const" UIDENT ":" type "=" expr NL ;

fn_decl       = "fn" LIDENT [ generics ] "(" [ params ] ")" [ "->" type ]
                { clause } block ;
generics      = "[" generic { "," generic } [ "," ] "]" ;
generic       = UIDENT [ ":" UIDENT { "+" UIDENT } ] ;
params        = param { "," param } [ "," ] ;
param         = ( "self" | LIDENT ":" type ) ;
clause        = "needs" cap_list
              | "requires" expr
              | "ensures" expr ;
cap_list      = UIDENT { "," UIDENT } ;

type_decl     = [ "opaque" ] "type" UIDENT [ generics ] "=" type_body NL ;
type_body     = record_body
              | variant { "|" variant }
              | type ;
record_body   = "{" field { "," field } [ "," ] "}" ;
field         = LIDENT ":" type ;
variant       = UIDENT [ "(" variant_fields ")" ] ;
variant_fields= ( field | type ) { "," ( field | type ) } [ "," ] ;

trait_decl    = "trait" UIDENT [ generics ] "{" { fn_sig NL } "}" ;
fn_sig        = "fn" LIDENT [ generics ] "(" [ params ] ")" [ "->" type ] { clause } [ block ] ;
impl_decl     = "impl" [ generics ] UIDENT [ type_args ] "for" type "{" { fn_decl } "}"
              | "impl" [ generics ] type "{" { [ "pub" ] fn_decl } "}" ;   (* inherent methods *)

cap_decl      = "cap" UIDENT "{" { fn_sig NL } "}" ;

test_decl     = "test" STRING block ;
```

## Types

```ebnf
type          = union_type [ "where" expr ] ;
union_type    = base_type { "|" base_type } ;          (* only valid where a tag union is allowed, see 06-errors.md *)
base_type     = UIDENT [ type_args ]                    (* Int, List[T], Result[T, E] *)
              | UIDENT "(" variant_fields ")"           (* inline tag with payload: Timeout(after: Int) *)
              | "(" ")"                                 (* Unit *)
              | "(" type "," type { "," type } ")"      (* tuple *)
              | record_body                             (* anonymous record type *)
              | "fn" "(" [ type { "," type } ] ")" "->" type [ "needs" cap_list ]
              | "block" "(" [ type { "," type } ] ")" "->" type ;   (* parameter position only *)
type_args     = "[" type { "," type } [ "," ] "]" ;
```

## Statements and blocks

```ebnf
block         = "{" { stmt } [ expr ] "}" ;           (* value of a block is its final expression, else Unit *)
stmt          = "let" pattern [ ":" type ] "=" expr NL
              | "var" LIDENT [ ":" type ] "=" expr NL
              | place assign_op expr NL
              | "while" expr_nb { "invariant" expr } block
              | "for" pattern "in" expr_nb { "invariant" expr } block
              | "return" [ expr ] NL
              | "break" NL | "continue" NL
              | "assert" expr [ "," STRING ] NL
              | expr NL ;
assign_op     = "=" | "+=" | "-=" | "*=" | "/=" ;
place         = LIDENT { "." LIDENT | "[" expr "]" } ;
```

`expr_nb` ("no block") is an expression in which a trailing block is not allowed at the top level. It is used in `if`, `while`, `for`, and `match` heads so that `if ready { ... }` parses the `{` as the body, not as a block argument to `ready`. Parenthesize to pass a block in a head: `if (xs.any? { |x| x > 3 }) { ... }`.

## Expressions

```ebnf
expr          = pratt_expr ;                           (* see precedence table *)

primary       = literal
              | LIDENT | UIDENT
              | "self" | "result"
              | "(" expr ")"
              | "(" expr "," expr { "," expr } ")"      (* tuple *)
              | "[" [ expr { "," expr } [ "," ] ] "]"   (* list literal *)
              | UIDENT "{" field_init { "," field_init } [ "," ] "}"   (* record construction *)
              | block
              | if_expr | match_expr
              | lambda
              | handle_expr | scope_expr
              | "spawn" expr ;

field_init    = LIDENT [ ":" expr ] ;                  (* `{ cents }` shorthand for `{ cents: cents }` *)

if_expr       = "if" expr_nb block [ "else" ( if_expr | block ) ] ;
match_expr    = "match" expr_nb "{" { arm } "}" ;
arm           = pattern [ "if" expr ] "=>" ( expr | block ) [ "," ] ;

lambda        = "|" [ lparams ] "|" expr               (* first-class closure *)
              | "fn" "(" [ params ] ")" [ "->" type ] block ;
block_arg     = "{" "|" [ lparams ] "|" { stmt } [ expr ] "}"   (* trailing block (second-class) *)
              | "{" { stmt } [ expr ] "}" ;            (* zero-param trailing block *)
lparams       = LIDENT [ ":" type ] { "," LIDENT [ ":" type ] } ;

handle_expr   = "handle" UIDENT "with" expr block ;
scope_expr    = "scope" block ;

postfix       = primary { call_suffix | "." LIDENT [ call_suffix ] | "." INT | "[" expr "]" | "?" } ;
call_suffix   = "(" [ args ] ")" [ block_arg ]
              | block_arg ;                            (* `xs.each { |x| ... }` with no parens *)
args          = arg { "," arg } [ "," ] ;
arg           = [ LIDENT ":" ] expr ;                  (* named arguments allowed *)
```

### Precedence (lowest to highest)

| Level | Operators | Associativity |
|---|---|---|
| 1 | `=>` (implication, contracts only) | right |
| 2 | `\|\|` | left |
| 3 | `&&` | left |
| 4 | `==` `!=` `<` `<=` `>` `>=` | non-associative |
| 5 | `..` `..=` | non-associative |
| 6 | `\|>` | left |
| 7 | `+` `-` | left |
| 8 | `*` `/` `%` | left |
| 9 | prefix `-` `!` | prefix |
| 10 | postfix: call, `.field`, `.method`, `[i]`, `?` | left |

`=>` as an operator is only legal inside `requires`, `ensures`, `invariant`, `assert`, and `where` predicates (diagnostic `PT0110` elsewhere). In match arms `=>` is punctuation.

## Patterns

```ebnf
pattern       = "_"
              | LIDENT                                  (* binding *)
              | literal
              | UIDENT [ "(" pattern { "," pattern } ")" ]          (* constructor / tag *)
              | UIDENT "{" field_pat { "," field_pat } [ "," ".." ] "}"
              | "(" pattern "," pattern { "," pattern } ")"
              | "[" [ pattern { "," pattern } [ "," ".." LIDENT ] ] "]"
              | pattern "|" pattern                     (* or-pattern *)
              | literal ( ".." | "..=" ) literal ;      (* range *)
field_pat     = LIDENT [ ":" pattern ] ;
```

## Method call sugar (uniform function call syntax)

`x.f(a, b)` resolves to, in order:
1. a field `f` of `x` (then called if it is a function value),
2. a method `f` in an inherent `impl` for the type of `x`,
3. a trait method `f` implemented for the type of `x`,
4. a function `f` in scope whose first parameter accepts the type of `x` (so `x.f(a)` is `f(x, a)`).

A zero-argument call may omit parentheses: `invoices.sum` is `sum(invoices)`. If a field and a method share a name, the field wins and lint `PT1042` warns.
