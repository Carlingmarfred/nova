# Nova Natural — Normative Grammar (EBNF)

**Status:** NORMATIVE for the native pipeline (N-series). This document is the written
contract that the Rust lexer/parser must implement (closes open question Q12). It was
extracted from the bootstrap implementation (`bootstrap/nova_lexer.py`,
`bootstrap/nova_parser.py`), which passes all 236 suite tests and 20 golden AST dumps.
Where this document and the bootstrap ever disagree, **fix one of them before writing
native code** — goldens are the byte-compatibility target (`nova_dump.py` format).

Notation: `~` = exception, `?` = optional, `*` = zero or more, `+` = one or more,
`,` = concatenation, `|` = alternation. Quoted strings are literal WORDs
(matched case-insensitively — see §2.4). The compact-skin symbol spellings are given
in brackets `[...]` beside their word forms; both produce identical AST nodes.

---

## 1. Lexical grammar

### 1.1 Source file

```ebnf
file           = [ BOM ] , [ shebang ] , { line_content } ;
BOM            = U+FEFF ;                       (* tolerated, skipped *)
shebang        = "#!" , { any_char_except_newline } ;   (* line 1 only *)
```

Encoding is UTF-8. Comments run to end of line: `#` or `//`.
A `;` produces a NEWLINE token (statement separator), so one physical line may hold
several sentences. Blank lines produce NEWLINEs; they are skipped everywhere except
inside string literals.

### 1.2 Tokens

```ebnf
token          = WORD | NUMBER | STRING | NEWLINE | EOF | punct ;

NEWLINE        = "\n" | ";" ;
punct          = "(" | ")" | "[" | "]" | ","            (* structural *)
               | skin_symbol ;
skin_symbol    = "=" | "+" | "-" | "*" | "/" | "%" | "<" | ">" | "!" | "." 
               | "{" | "}" | "?" | "==" | "!=" | "<=" | ">=" | "&&" | "||" ;
```

Two-character symbols are recognized before one-character ones. `{` and `}` are
lexed but have **no parser production yet** (reserved for future blocks/lambdas);
they raise `unexpected '{'/'}' in an expression` if used today.

### 1.3 Numbers

```ebnf
NUMBER         = digits , [ "." , digits ] ;
digits         = digit , { digit | "_" } ;      (* "_" separators allowed *)
```

- A fractional part requires at least one digit after the dot (`1.` is the integer 1;
  `.5` is not a number).
- **No exponent notation** (`1e5` is a syntax error today).
- Integer literals are arbitrary-precision; fractionals are binary floats
  (see README decision log, integer-model amendment 2026-08-24).

### 1.4 Words

```ebnf
WORD           = word_start , { word_continue } ;
word_start     = alphabetic | "_" ;
word_continue  = alphabetic | digit | "_" | "-"~digit_guard ;
digit_guard    : "-" continues a word ONLY if the next character is NOT a digit
                 ("-letter"/"-_" continue; "-digit" ends the word so MINUS wins);
                 trailing "-" characters are stripped from the word.
```

Examples: `guess-count` is one WORD; `x-1` lexes as WORD(`x`) MINUS NUMBER(1).
Words are matched **case-insensitively** for keywords/phrases; identifier bindings
keep their original casing (`Name` ≠ `name` as variables).

### 1.5 Strings

```ebnf
STRING         = quote , { char_without_newline_or_quote | escape } , quote ;
quote          = '"' | "'" ;
escape         = "\" , ( "n" | "t" | "\" | quote | "{" | "}" ) ;
```

A raw newline inside a string is an error. `\{` and `\}` emit literal braces; an
**unescaped** `{` starts an interpolation hole whose content is captured verbatim and
re-parsed at RUNTIME (open question Q9 — interpolation-as-AST is parked; the native
lexer must preserve holes exactly like the bootstrap does).

### 1.6 Reserved words

These WORDs cannot be used where `expect_name(what)` applies (variable names, function
names, parameter names, thing names, field names, module member positions, loop
variables, `as`-error names):

```ebnf
reserved       = "say" | "write" | "if" | "unless" | "repeat" | "stop" | "skip"
               | "go" | "set" | "add" | "take" | "remove" | "check" | "try" | "to"
               | "use" | "wait" | "pause" | "track" | "undo" | "redo" | "exit"
               | "when" | "requires" | "ensures" | "give" | "return" | "store"
               | "then" | "otherwise" | "done" | "is" | "and" | "or" | "not"
               | "the" | "of" | "in" | "from" | "with" | "a" | "an"
               | "true" | "false" | "nothing" | "none" | "null"
               | "ask" | "every" | "everything" | "item" | "how" | "many"
               | "it" ;
```

**Exemption:** names AFTER `.` (attribute access, e.g. `file.write`) skip the
reserved check. Dotted module calls likewise.

---

## 2. Program structure

```ebnf
program        = { NEWLINE } , { statement , { NEWLINE } } , EOF ;
```

Statements end at NEWLINE (or `;`). Every multi-line construct terminates with its own
`done` — there is **no indentation sensitivity** anywhere.

### 2.1 Statement dispatch (first word decides)

```ebnf
statement      = use_stmt | say_stmt | if_stmt | repeat_stmt | stop_stmt
               | skip_stmt | set_stmt | add_stmt | take_stmt | remove_stmt
               | check_stmt | try_stmt | func_def | thing_def | wait_stmt
               | pause_stmt | track_stmt | undo_stmt | exit_stmt
               | program_starts | contract_stmt | return_stmt | store_stmt
               | usemodule_stmt | assign_compact | declare_assign | expr_stmt ;
```

Exact trigger sequences (LH = lookahead requirement):

| Production | Trigger words | Notes |
|---|---|---|
| `use_stmt` | `use …` | rest of line consumed VERBATIM into `UseLib.text`; validated at runtime |
| `say_stmt` | `say` / `write` | `say` sets newline flag true, `write` false |
| `if_stmt` | `if` / `unless` | `unless` negates the first condition |
| `repeat_stmt` | `repeat` | see §3.3 |
| `stop_stmt` | `stop` | `stop [the] loop` → Break, `stop [the] program` → StopProgram |
| `skip_stmt` | `skip this one` / `go to next turn` | both → ContinueStmt |
| `set_stmt` | `set` | target per §3.6 |
| `add_stmt` | `add` | `add E to [the] NAME` → AddTo |
| `take_stmt` | `take` | `take E from [the] NAME` → TakeFrom |
| `remove_stmt` | `remove` | `remove E` (E = arith; runtime restricts to item-form) |
| `check_stmt` | `check` | see §3.4 |
| `try_stmt` | `try` | see §3.5 |
| `func_def` | `to` | see §3.7 |
| `thing_def` | `a`/`an` + LH: word+2 = `is`, word+4 = `thing` | `a NAME is a thing with FIELD* done` |
| `wait_stmt` | `wait` | `wait AMOUNT UNIT` |
| `pause_stmt` | `pause the program` | PauseProgram |
| `track_stmt` | `track [the] NAME` | TrackStmt |
| `undo_stmt` | `undo`/`redo the last change to [the] NAME` | UndoStmt(redo flag) |
| `exit_stmt` | `exit` | StopProgram |
| `program_starts` | `when the program starts` LH exact | block + `done` |
| `contract_stmt` | `requires` / `ensures` | `Contract(kind, expr)` |
| `return_stmt` | `give`/`return` [+ `back`] | expression optional |
| `store_stmt` | `store` | `store ARITH in ARITH [as json]` → StoreJson |
| `usemodule_stmt` | `the` + LH: word+1 is WORD, word+2 = `in` | see §3.8 |
| `assign_compact` | WORD + LH: `=` or dotted path ending in `=` | `NAME(.FIELD)* = EXPR` |
| `declare_assign` | `[the] [my] NAME is EXPR` | both articles optional, in that order |
| `expr_stmt` | anything else (must be a call) | ExprStmt |

### 2.2 Blocks and bodies

```ebnf
block(stop)    = { NEWLINE } , { statement , { NEWLINE } } , stop_word ;
body(stop)     = NEWLINE , block(stop)        (* multi-line; reports used_done=true *)
               | inline_stmt ;                (* exactly ONE statement, ends at NEWLINE *)
```

- `block(stop)` fails with "the block is never closed — expected '<stops>'" at EOF.
- An inline body is a single statement; a later-line `otherwise` after an inline `then`
  body is therefore impossible — use the full block form when any branch is multiline.
- Stop words per construct: if-chain bodies stop at `otherwise`/`done`; check arms at
  `when`/`otherwise`/`done`; try bodies at `if`/`done` (**known limitation:** a nested
  statement beginning with the word `if` cannot sit directly inside a try body);
  everything else stops at `done`.

### 2.3 done-tolerance (if-chains only)

If any branch body was a newline block, the chain MUST end with `done`. If ALL branches
were inline, a trailing `done` is tolerated (consumed silently) but not required.

---

## 3. Statements in detail

### 3.1 Output

```ebnf
say_stmt       = ("say"|"write") , expression , { "and" , expression } ;
```

Multiple expressions print back-to-back on one output event; `say` appends a newline.

### 3.2 Conditional — if / otherwise

```ebnf
if_stmt        = ("if"|"unless") , expression , "then" , body({"otherwise","done"})
               , { "otherwise" , "if" , expression , "then" , body({"otherwise","done"}) }
               , [ "otherwise" , body({"done"}) ] , [ "done"~tolerated ] ;
```

Semantics: first true branch wins; `unless` wraps condition 1 in `not`.

### 3.3 Loops — repeat

```ebnf
repeat_stmt    = "repeat" , "forever" , block({"done"}) , "done"
               | "repeat" , ("until"|"while") , expression , NEWLINE , block({"done"}) , "done"
               | "repeat" , [ "for" ] , "each" , NAME , "in" , expression , NEWLINE ,
                 block({"done"}) , "done"
               | "repeat" , "with" , NAME , "from" , count_expr , "to" , count_expr , NEWLINE ,
                 block({"done"}) , "done"
               | "repeat" , count_expr , "times" , NEWLINE , block({"done"}) , "done" ;
count_expr     = factor , { ("plus"|"minus") , factor } ;   (* 'times' is NOT an operator here *)
```

### 3.4 Multi-way match — check

```ebnf
check_stmt     = "check" , expression , { NEWLINE } ,
                 { "when" , [ "it" ] , [ "is" ] , pattern , body({"when","otherwise","done"}) }
                 , [ "otherwise" , body({"when","done"}) ] , [ "done" ] ;

pattern        = [ "not" ] , pattern_kind ;
pattern_kind   = [ "a"|"an" ] , "number"                    (* isnum *)
               | "equal" , "to" , arithmetic                (* eq *)
               | "the" , "same" , "as" , arithmetic         (* eq *)
               | "starts" , "with" , arithmetic             (* startswith *)
               | "ends" , "with" , arithmetic               (* endswith *)
               | "contains" , arithmetic
               | "empty"
               | arithmetic                                 (* default: eq *)
```

`when it is …` / bare `when <pattern>` both work (`it`/`is` are eaten if present).
The subject expression is evaluated once. Unmatched subject falls through silently
when no `otherwise` arm exists (exhaustiveness lint = item C11).

### 3.5 Errors — try

```ebnf
try_stmt       = "try" , block({"if","done"}) ,
                 [ "if" , "it" , "fails" , [ "as" , NAME ] , block({"done"}) ] , "done" ;
```

The handler catches NovaErrors raised inside the body; with `as ERR` the error message
is bound to `ERR` for the handler scope.

### 3.6 Assignment targets

```ebnf
set_stmt       = "set" , lvalue , "to" , expression ;
lvalue         = the_chain_field          (* must be Field form, else sentence error *)
               | [ "my" ] , NAME , [ "of" , arithmetic→field_form ] ;
assign_compact = NAME , { "." , NAME } , "=" , expression ;
declare_assign = [ "the" ] , [ "my" ] , NAME , "is" , expression ;
```

Notes: `set the F of OBJ to V` is the canonical field write; `NAME.of OBJ` style via
lvalue's `of`-branch reads `NAME` as the FIELD of `OBJ`. `my x is E` declares.

### 3.7 Definitions

```ebnf
func_def       = "to" , NAME , [ "with" , NAME , { "and" , NAME } ] , NEWLINE ,
                 block({"done"}) , "done" ;

thing_def      = ("a"|"an") , NAME , "is" , "a" , "thing" , "with" ,
                 { NEWLINE } , { field_default , { NEWLINE } } , "done" ;
field_default  = [ "a"|"an" ] , NAME , [ "set" , "to" , expression ] ;
                 (* without "set to": field ends at NEWLINE, default = none *)

program_starts = "when" , "the" , "program" , "starts" , NEWLINE ,
                 block({"done"}) , "done" ;

contract_stmt  = ("requires"|"ensures") , expression ;
                 (* requires evaluated eagerly at call, ensures at exit *)
```

### 3.8 Modules and libraries

```ebnf
usemodule_stmt = "the" , mod_name , "in" , STRING ;
mod_name       = WORD ending in "-module" ;        (* enforced at parse time *)

use_stmt       = "use" , rest_of_line_verbatim ;   (* stdlib form validated at runtime:
                                                      use [the] standard NAME [library] *)
module_call    = Var , "." , NAME , "(" , [ arg_list ] , ")" ;
                 (* ONLY when the receiver is a bare variable; any other base raises
                    the friendly "arrives in a later version" sentence *)
```

Imported modules bind under their full name including the `-module` suffix.

### 3.9 History — track / undo / redo

```ebnf
track_stmt     = "track" , [ "the" ] , NAME ;
undo_stmt      = ("undo"|"redo") , "the" , "last" , "change" , "to" , [ "the" ] , NAME ;
```

### 3.10 Misc

```ebnf
wait_stmt      = "wait" , count_expr , time_unit ;
time_unit      = "second"|"seconds"|"minute"|"minutes"|"hour"|"hours"
               |"ms"|"millisecond"|"milliseconds" ;
return_stmt    = ("give"|"return") , [ "back" ] , [ expression ] ;
store_stmt     = "store" , arithmetic , "in" , arithmetic , [ "as" , "json" ] ;
remove_stmt    = "remove" , arithmetic ;
skip_stmt      = "skip" , "this" , "one" | "go" , "to" , "next" , "turn" ;
stop_stmt      = "stop" , [ "the" ] , ( "loop" | "program" ) ;
exit_stmt      = "exit" ;
pause_stmt     = "pause" , "the" , "program" ;
```

---

## 4. Expressions

Precedence, lowest to highest. Word forms and bracketed symbol forms share ONE
grammar and produce identical op strings in the AST:

```ebnf
expression     = optional_wrap( or_expr ) ;        (* §4.8 *)

or_expr        = and_expr , { ( "or" | ["||"] ) , and_expr } ;
and_expr       = cmp_expr , { ( "and" | ["&&"] ) , cmp_expr } ;

cmp_expr       = arith , { comparison_tail } ;     (* LEFT-ASSOCIATIVE, chainable — see note *)
comparison_tail= ["=="] , arith                                    (* eq  *)
               | ["!="] , arith                                    (* ne  *)
               | ["<"]  , arith                                    (* lt  *)
               | ["<="] , arith                                    (* lte *)
               | [">"]  , arith                                    (* gt  *)
               | [">="] , arith                                    (* gte *)
               | "is" , is_rhs
               | "contains" , arith
               | "starts" , "with" , arith
               | "ends" , "with" , arith
               | "has" , "no" , "items"
               | "exists"
               | "does" , "not" , "exist" ;

is_rhs         = [ "not" ] , is_kind ;
is_kind        = [ "a"|"an" ] , "number"           (* IsNumberTest(negate) *)
               | "nothing"                          (* eq/ne Lit(nothing) *)
               | "empty"                            (* IsEmptyE, negation wraps not *)
               | "true" | "false"                   (* eq/ne bool literal *)
               | "equal" , "to" , arith
               | "the" , "same" , "as" , arith
               | "greater" , "than" , arith         (* negate → lte *)
               | "less" , "than" , arith            (* negate → gte *)
               | "at" , ( "least"→gte | "most"→lte ) , arith   (* negate swaps again *)
               | arith                              (* default eq; negate → ne *)

arith          = term , { ( "plus"|["+"] | "minus"|"[-]") , term } ;
term           = factor , { mul_tail } ;
mul_tail       = ( "times"|"[*]" ) , factor
               | "multiplied" , "by" , factor
               | ( "divided" , "by" | "over" | ["/"] ) , factor
               | ( "mod" | [%] ) , factor ;

factor         = "not" , cmp_expr                  (* word-not binds a COMPARISON *)
               | [!] , factor                      (* bang binds a FACTOR *)
               | [-] , factor                      (* desugars to minus(Lit(0), factor) *)
               | postfix ;

postfix        = primary , { postfix_op } ;
postfix_op     = "." , attr_name , [ "(" , [ arg_list ] , ")" ]   (* Field; call = §3.8 rule *)
               | "?"                                              (* Optional marker, §4.8 *)
arg_list       = arithmetic , { "," , arithmetic } ;
attr_name      = WORD , NO reserved-check ;
```

> **Note — comparison tails (verified against bootstrap):** the tail list iterates
> left-associatively, so tails may repeat: `1 < 2 < 3`, `a is less than b is less than c`
> and `"abc" contains "b" contains "c"` all chain. BUT a bare comparative phrase without
> its second `is` does NOT continue a chain (`a less than b less than c` is a sentence
> error) — each word-form tail after the first must be reintroduced by `is` (or be
> `contains`/`starts with`/`ends with`). The full-language spec says comparisons do not
> chain at all; bootstrap behavior is PINNED for v0.2 byte-compat, lint flagged for C11/D03.

> **Note — asymmetric not:** `not X` parses X as a full comparison while `!X`
> parses X as a factor. Kept deliberately (sentence rhythm); equivalence pairs pin it.

### 4.1 Primary expressions

```ebnf
primary        = NUMBER | STRING
               | "(" , expression , ")"
               | list_literal
               | "true" | "false" | ( "nothing"|"none"|"null" → Lit(nothing) )
               | "ask" , arithmetic                                   (* AskE *)
               | ("a"|"an") , "random" , "number" , "between" ,
                 count_expr , "and" , count_expr                      (* RandomBetween *)
               | ("a"|"an") , "empty" , "list"                        (* EmptyListE *)
               | ("a"|"an") , "copy" , "of" , arithmetic              (* CopyOf *)
               | ("a"|"an") , "new" , NAME , [ setter_list ]          (* NewThing *)
               | "how" , "many" , "items" , "are" , "in" , factor     (* CountOf *)
               | "everything" , "after" , arithmetic , "in" , [ "the" ] ,
                 arithmetic                                           (* EverythingAfter *)
               | "every" , "item" , "of" , arithmetic , "turned" , "into" ,
                 [ "a"|"an" ] , WORD                                  (* EveryTurnedInto *)
               | "item" , count_expr , "of" , arithmetic              (* ItemAt *)
               | "the" , the_chain                                    (* §4.2 *)
               | "not" , cmp_expr
               | bare                                                 (* §4.3 *) ;

list_literal   = "[" , [ expression , { "," , expression } ] , "]" ;
setter_list    = "with" , setter , { "and"~requires_"set"_ahead , setter } ;
setter         = WORD , "set" , "to" , arithmetic ;
bare           = NAME , ( "(" , [ arg_list ] , ")"                     (* Call *)
                        | "with" , arithmetic , { "and" , arithmetic } (* Call *)
                        | ε                                            (* Var *) ) ;
```

### 4.2 The `the … of` chain (value phrases)

```ebnf
the_chain      = "the" , chain_head ;
chain_head     = "contents" , [ "of" ] , arithmetic , [ "parsed" , "as" , "json" ]
               | "first" , "item" , "of" , factor
               | "last"  , "item" , "of" , factor
               | "number" , "value" , "of" , factor
               | "length" , [ "of" ] , factor
               | NAME , "of" , arithmetic                (* Field read *)
               | NAME                                    (* bare Var fallback *)
```

Factor-level binding (first/last/length/number-value/how-many) is DELIBERATE:
`(the number value of x?) plus 1` must parse as `plus(NumVal(x?), 1)` so the `?`
poison sees the conversion. Phrases with trailing keyword clauses
(contents/every-item) stay greedy.

### 4.3 Names, calls, fields

- `NAME(arg, arg)` → Call. `NAME with a and b` → Call([a, b]).
- `x.f.g` chains Field(Field(x,f),g); reserved words do NOT apply after dots.
- `module.fn(args)` is legal ONLY on a bare variable receiver.

### 4.4 Built-in phrase heads win over fields

`the length of x` is the built-in CountOf even if a thing has a field named `length`;
use dotted `x.length` to reach such a field (known v0.15 behavior, kept).

## 4.8 Optional `?` — whole-expression poisoning (C03)

Parsing marks every postfix `?` as a local `QuestionE` marker. When the finished tree
for ONE `expression` contains any marker, ALL markers are stripped and the entire tree
is wrapped exactly ONCE at the root. Consequences:

- Dumped ASTs never contain nested `QuestionE`.
- Parenthesized sub-expressions re-enter `expression`, but the outer wrap still
  collapses to a single root wrapper.
- Without any `?`, evaluation errors stay loud (only absence-of-value propagates).

---

## 5. AST node contract (dump targets)

Native parser must produce these node names, field-for-field, in `nova_dump.py`'s
line-based format (2-space indent per depth, `field:` prefixes, `[N]` lists,
`{N} keys` dicts, Python-`repr` scalars — see `bootstrap/nova_dump.py` header).

**Statements:** `Say(exprs,newline)` · `Assign(target,expr)` · `AddTo(name,expr)` ·
`TakeFrom(name,expr)` · `If(branches,otherwise)` · `Block(stmts)` ·
`RepeatForever(body)` · `RepeatUntil(cond,body)` · `RepeatWhile(cond,body)` ·
`RepeatTimes(count,body)` · `RepeatEach(var,iterable,body)` ·
`RepeatCounting(var,start,end,body)` · `BreakStmt` · `ContinueStmt` · `StopProgram` ·
`PauseProgram` · `Check(subject,arms,otherwise)` · `TryStmt(body,errname,handler)` ·
`FuncDef(name,params,body)` · `ThingDef(name,fields{dict})` · `ReturnStmt(expr)` ·
`WaitStmt(amount,unit)` · `UseLib(text)` · `UseModule(name,path)` · `TrackStmt(name)` ·
`UndoStmt(name,redo)` · `Contract(kind,expr)` · `RemoveStmt(expr)` ·
`StoreJson(value,path)` · `ExprStmt(expr)` · `WhenProgramStarts(body)`

**Expressions:** `Lit(value)` · `StrLit(raw)` · `ListLit(items)` · `EmptyListE` ·
`Var(name)` · `Field(obj,name)` · `Bin(op,l,r)` · `NotE(e)` · `Call(name,args)` ·
`ModuleCall(mod,name,args)` · `NewThing(cls,setters[tuples])` · `NumVal(e)` ·
`EverythingAfter(sep,e)` · `CountOf(e)` · `ItemAt(idx,e)` · `FirstItem(e)` ·
`LastItem(e)` · `IsEmptyE(e)` · `HasNoItems(e)` · `ExistsE(e,flag)` ·
`IsNumberTest(e,negate)` · `RandomBetween(a,b)` · `ContentsOf(e,as_json)` ·
`EveryTurnedInto(e,thing)` · `CopyOf(e)` · `AskE(prompt)` · `QuestionE(e)` (root-only)

Op strings in `Bin.op`: `or and eq ne lt lte gt gte plus minus times divided mod
contains startswith endswith`. If-arm tuples are `(cond, Block)` pairs; check-arm
tuples are `(kind, val, negate, Block)` with kind ∈
`eq isnum startswith endswith contains isempty`.

Scalar reprs follow Python `repr()` exactly: strings single-quoted with escapes
(`'hi'`), booleans `True`/`False`, nothing `None`, ints plain, floats Python-style
(`3.5`, `1.0`). The native dumper must replicate these byte-for-byte.

---

## 6. Known quirks pinned for v0.2 (do NOT "fix" silently)

1. Comparison tails chain left-associatively (§4 note).
2. `not` vs `!` operand asymmetry (§4 note).
3. Unary minus desugars to `minus(Lit(0), factor)`.
4. `wait`/`repeat N times`/counting/random-bounds/item-index use `count_expr`
   (`times` is not multiplicative there).
5. `the NAME` falls back to a bare `Var` when no `of` follows.
6. `use` swallows the rest of the line verbatim; malformed library forms fail at
   runtime with the `stdlib.use_form` sentence.
7. Thing-name after `every item of X turned into a` is NOT reserved-checked
   (inconsistent with `new`); pinned for byte-compat, revisit post-v0.2.
8. Try bodies cannot host statements starting with the bare word `if` (§2.2).
9. `{`/`}` lex but have no productions yet.
10. String interpolation content re-lexes at runtime (Q9 parked).
```
