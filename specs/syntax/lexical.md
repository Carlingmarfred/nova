# Nova Lexical Rules

## Filformat

- Encoding: UTF-8 (BOM tolereret). LF og CRLF accepteres, normaliseres til `\n`.
- Filendelse: `.nova`.

## Kommentarer

```text
// linjekommentar
/* blokkommentar /* kan nestes */ */
/// doc-kommentar (linje)
/** doc-blok **/
```

Kommentarer sendes i trivia-streamen til formatter/LSP — parseren ignorerer dem.

## Newline-semantik (statement-slut)

Lexer producerer `NEWLINE`-tokens. Parserens regel:

Et statement afsluttes ved NEWLINE **medmindre**:
1. Linjen slutter med binær operator, `,`, `(`, `[`, `{`, `=>`, `->`, `.`, `?.`, `=` (assignments op), `where`, `||`, `&&`
2. Næste linje starter med lukke-token (`)`, `]`, `}`), binær operator, `.`, `?.`, eller `catch`/`finally`/`else`

Ellers er `;` eksplicit separator. Denne regel er deterministisk og implementeret i lexeren — ingen JS-agtig tvetydig ASI.

## Identifiers

```text
IDENT    = (XID_Start | "_") XID_Continue*    -- Unicode identifiers tilladt
SCREAMING_CASE anbefales for consts (linter), ingen case-regel i grammatikken
```

Raw identifier: `r#"match"#` hvis man skal skygge et keyword (FFI).

## Literals

### Heltal

```text
42        → i32
0x_FF     hex     0o777   oktal    0b1010  binær
42u8  42i64  42usize            suffixed
10_000_000                   underscores frit
99999999999999999999999n     BigInt-literal
```

Hvis literal overstiger i32-range uden suffix: compiler-fejl (kræv suffix/L BigInt). Ingen stiltiende promotion.

### Floats

```text
3.14 → f64    3.14f32    1e10    1.5e-9    inf    nan    0x1.8p3 (hex-float)
```

### char / String

```text
'a' '\n' '\u{1F600}'             char (Unicode scalar)
"Hej"                            String (UTF-8)
"interpoleret {name} og {x + 1}"
"{pris:.2} kr"                   format-spec: fill align sign width .precision type (Python-kompatibel)
r"C:\raw\path"                   raw string
"""multi
line"""                          multiline
b"bytes"                         Bytes
"\n\t\\\"\{\'\0\u{7F}"           escapes
```

Interpolation `{expr}` gælder KUN i almindelige `"`-strings (ikke raw/multiline uden prefix `f`... beslutning: interpolation aktiv i `"` og `"""`; brug `r"` for at slå fra; `{{` og `}}` escapes).

### bool / none

`true false none null(unsafe)`.

## Operator-tokens (komplet)

```text
+ - * / % // ** = += -= *= /= %= //= **= &= |= ^= <<= >>=
== != < <= > >= <=> && || ! ~ & | ^ << >> ? ?? ?. ?=
.. ..< ..= -> => :: . , : ; ( ) [ ] { }
@ # $ (kun i makroer)
```

### Bootstrap-udsnit (v0.11+, item C01)

Bootstrap-lexeren genkender nu symbol-delmængden og parseren afbilder den til de
SAMME AST-operatorstrenge som Natural (én AST, to skins):

```text
=      → tildeling (som "is" / "set ... to")
+ - * / %  → plus / minus / times / divided / mod
== !=  → eq / ne       < <= > >=  → lt / lte / gt / gte
&& || ! → and / or / not        .navn → Field (som "the navn of ...")
( ) [ ] , ; . { }  → som forventet; { } lexes rent men har ENDNU ingen
                     statement-grammatik (lambdas/fn kommer i C10/T3)
```

**Bindestreg-policy (kilde til sandhed):** inde i et ord fortsætter `-` ordet når
næste tegn er et bogstav/`_`/`-` (`save-file`); følges `-` af et ciffer, afsluttes
ordet og `-` lexes som MINUS (`x-1` = `x minus 1`). En selvstændig `-` er altid
MINUS. Dermed er `a-b` ét navn, `a -b`, `a- b` og `a - b` alle subtraktion.

**Præcedens (lav→høj), fælles for begge skins:**

```text
or / ||   <   and / &&   <   sammenligning (is..., ==, !=, <, <=, >, >=)
<   plus/minus (+ -)   <   times/divided/mod (* / %)
<   unær (!, -)   <   postfix (.felt)   <   primærer
```

Symbol-operander blandes frit med ord-operatorer; `1 plus 2 * 3 == 6 + x && y`
parser deterministisk efter tabellen ovenfor.

## Keywords (reserverede, fuld liste)

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

Kontekstuelle (ikke reserverede som identifiers andre steder): `get set operator init deinit test expect compile macro base channel select spawn joined-with grouped-by turned-into`.

Unikke features (se ../unique_features.md): Flow<T> + Table (stdlib §23), undo/redo +
variabel-historik (`track`/`undo`/`ever`), tillids-sporing (taint), tilstandsmaskiner
(`states`/`becomes`/`waits`), `exact`-blokke, `every`/`in`-tidsudtryk, `@incremental`,
`nova why`, grammatik-literals.

## Shebang

`#!...` på første linje behandles som kommentar.

## Reserverede ord i Nova Natural (bootstrap v0.11)

I Natural-skinnen er følgende ord reserveret — de kan ikke bruges som variabelnavne,
parametre, funktions-/thing-navne eller feltnavne, fordi de enten starter sætninger,
forbinder klausuler eller indleder indbyggede udtryk:

```text
Sætnings-startere:
    say write if unless repeat stop skip go set add take remove check try to
    use wait pause track undo redo exit when requires ensures give return store

Struktur & konnektorer:
    then otherwise done is and or not the of in from with a an

Værdier:
    true false nothing none null

Indbyggede udtryks-hoveder:
    ask every everything item how many
```

**Policy:** kun ord der gør programmet *uparseligt* reserveres. Ord som `count`,
`mark`, `number`, `length`, `first`, `last`, `answer` er bevidst IKKE reserverede.
Bruger man et reserveret ord som navn, får man:

```text
Parser-fejl — linje L, kolonne C: 'done' er et reserveret ord og kan ikke bruges
som navn — vælg et andet navn
```

Kompakt-skin-nøgleordene i keyword-tabellen ovenfor (`fn let const …`) træder først i
kraft, når skindet implementeres (ITERATION_PLAN C01); de to skins deler AST og skal
dele den fulgte reservationspolitik ved merge.
