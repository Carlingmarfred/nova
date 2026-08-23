# Nova Lexical Rules

## File format

- Encoding: UTF-8 (BOM tolerated). LF and CRLF accepted, normalized to `\n`.
- File extension: `.nova`.

## Comments

```text
// line comment
/* block comment /* may nest */ */
/// doc comment (line)
/** doc block **/
```

Comments travel in the trivia stream for formatter/LSP — the parser ignores them.

## Newline semantics (statement end)

The lexer produces `NEWLINE` tokens. Parser rule:

A statement ends at NEWLINE **unless**:
1. The line ends with a binary operator, `,`, `(`, `[`, `{`, `=>`, `->`, `.`, `?.`, `=`
   (assignments), `where`, `||`, `&&`
2. The next line starts with a closing token (`)`, `]`, `}`), a binary operator, `.`,
   `?.`, or `catch`/`finally`/`else`

Otherwise `;` is an explicit separator. This rule is deterministic and implemented in
the lexer — no JS-style ambiguous ASI.

## Identifiers

```text
IDENT    = (XID_Start | "_") XID_Continue*    -- Unicode identifiers allowed
SCREAMING_CASE recommended for consts (linter), no case rule in the grammar
```

Raw identifier: `r#"match"#` when you must shadow a keyword (FFI).

## Literals

### Integers

```text
42        → i32
0x_FF     hex     0o777   octal    0b1010  binary
42u8  42i64  42usize            suffixed
10_000_000                   underscores free
99999999999999999999999n     BigInt literal
```

If a literal exceeds the i32 range without a suffix: compiler error (require a suffix
or BigInt). No silent promotion.

### Floats

```text
3.14 → f64    3.14f32    1e10    1.5e-9    inf    nan    0x1.8p3 (hex-float)
```

### char / String

```text
'a' '\n' '\u{1F600}'             char (Unicode scalar)
"Hi"                             String (UTF-8)
"interpolated {name} and {x + 1}"
"{price:.2} kr"                  format spec: fill align sign width .precision type (Python-compatible)
r"C:\raw\path"                   raw string
"""multi
line"""                          multiline
b"bytes"                         Bytes
"\n\t\\\"\{\'\0\u{7F}"           escapes
```

Interpolation `{expr}` applies ONLY in plain `"` strings (decision: interpolation active
in `"` and `"""`; use `r"` to turn it off; `{{` and `}}` escape braces).

### bool / none

`true false none null(unsafe)`.

## Operator tokens (complete)

```text
+ - * / % // ** = += -= *= /= %= //= **= &= |= ^= <<= >>=
== != < <= > >= <=> && || ! ~ & | ^ << >> ? ?? ?. ?=
.. ..< ..= -> => :: . , : ; ( ) [ ] { }
@ # $ (macros only)
```

### Bootstrap cut (v0.11+, item C01/C03)

The bootstrap lexer recognizes this symbol subset and the parser maps it onto the SAME
AST operator strings as Natural (one AST, two skins):

```text
=      → assignment (like "is" / "set ... to")
+ - * / %  → plus / minus / times / divided / mod
== !=  → eq / ne       < <= > >=  → lt / lte / gt / gte
&& || ! → and / or / not        .name → Field (like "the name of ...")
?      → OptionalGuard postfix (C03): removed at parse; the WHOLE expression is
         wrapped in QuestionE — see specs/error_handling.md §2.1
( ) [ ] , ; . { }  → as expected; { } lexes cleanly but has NO statement grammar yet
                     (lambdas/fn arrive in C10/T3)
```

**Hyphen policy (source of truth):** inside a word, `-` continues the word when the next
character is a letter/`_`/`-` (`save-file`); when `-` is followed by a digit the word
ends and `-` lexes as MINUS (`x-1` = `x minus 1`). A standalone `-` is always MINUS.
Thus `a-b` is one name while `a -b`, `a- b`, `a - b` are all subtraction.

**Precedence (low→high), shared by both skins:**

```text
or / ||   <   and / &&   <   comparison (is..., ==, !=, <, <=, >, >=)
<   plus/minus (+ -)   <   times/divided/mod (* / %)
<   unary (!, -)   <   postfix (? and .field)   <   primaries
```

Symbol operands mix freely with word operators; `1 plus 2 * 3 == 6 + x && y` parses
deterministically per the table above.

## Keywords (reserved, full list)

```text
fn let const var struct class enum trait impl extend mod import export from
pub priv protected static override virtual self super Self
if else while loop for in break continue return yield
match where as is and or not true false none null
async await parallel spawn select channel
try catch throw finally defer use unsafe owned weak dyn dynamic
init deinit operator test expect macro compile base
actor signal computed effect on send request reply requires ensures
then take bind undo redo track ever exact every states becomes waits
```

Contextual (not reserved as identifiers elsewhere): `get set operator init deinit test
expect compile macro base channel select spawn joined-with grouped-by turned-into`.

Unique features (see ../unique_features.md): Flow<T> + Table (stdlib §23), undo/redo +
variable history (`track`/`undo`/`ever`), taint tracking, state machines
(`states`/`becomes`/`waits`), `exact` blocks, `every`/`in` time expressions,
`@incremental`, `nova why`, grammar literals.

## Shebang

`#!...` on the first line is treated as a comment.

## Reserved words in Nova Natural (bootstrap v0.11+)

In the Natural skin the following words are reserved — they cannot be used as variable
names, parameters, function/thing names or field names, because they start sentences,
connect clauses, or head built-in expressions:

```text
Sentence starters:
    say write if unless repeat stop skip go set add take remove check try to
    use wait pause track undo redo exit when requires ensures give return store

Structure & connectors:
    then otherwise done is and or not the of in from with a an

Values:
    true false nothing none null

Built-in expression heads:
    ask every everything item how many
```

**Policy:** only words that would make a program *unparseable* are reserved. Words like
`count`, `mark`, `number`, `length`, `first`, `last`, `answer` are deliberately NOT
reserved. Using a reserved word as a name yields:

```text
Parser error — line L, column C: 'done' is a reserved word and cannot be used as a
name — choose a different name
```

Note: names after `.` are attribute access, not bindings — reserved words do not apply
there (e.g. `file.write` works).
