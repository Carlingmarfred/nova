# Nova Natural Syntax ("Nova Natural")

**Mål:** Man skal næsten kunne *sige* sin idé til et andet menneske, skrive den ned ord for ord — og have en app.

```text
Idé sagt højt:
    "Hvis spillerens liv er under 10, sig advarsel. Ellers sig alt er godt."

Nova Natural:
    if the health of the player is less than 10 then
        say "Advarsel!"
    otherwise
        say "Alt er godt."
    done
```

## 1. Principper

1. **Kontrolleret engelsk — ikke fri NLP.** Sproget er stadig en formel grammatik med faste fraser. Det der ligner "engelsk" er et fast ordforråd på ~120 ord i faste mønstre. Parseren er deterministisk; ingen AI-tolkning.
2. **Én AST, to skins.** Nova Natural er den primære overflade. Den kompakte symbolsyntax (`{}`, `=>`, `+`, `>`, `print`) forbliver gyldig som ekspert-stenografi. Begge former kompilerer til **identisk AST** — man kan blande frit.
3. **Blokke lukkes med `done`.** Ikke indentation-følsomt (formatteren anbefaler indrykning, parseren kræver det ikke). Éntydigt og copy-paste-sikkert. Enkeltlinjes-former bruger `then`/`:` uden `done`.
4. **Støjord tilladt:** `the a an of it that value are is` kan bruges hvor de lyder naturligt: `the length of xs` ≡ `length of xs`. Parseren ignorerer dem.
5. **Fejlmeddelelser foreslår:** skriver man `display x`, svarer compileren *"Did you mean `say x`?"* (Levenshtein over frase-ordbogen).

## 2. Kommando-ordbogen (fuld tabel)

### Output og input

| Naturlig | Kompakt | Betydning |
|---|---|---|
| `say "hej"` | `print("hej")` | udskriv + linjeskift |
| `write "hej"` | `print(..., newline = false)` | udskriv uden linjeskift |
| `ask "Navn?" and remember it as name` | `name = stdin.line("Navn?")` | spørg og gem svaret |
| `say x and y` | `print(x, y)` | flere værdier |

### Variabler

| Naturlig | Kompakt |
|---|---|
| `x is 10` | `x = 10` (opret) |
| `set x to 20` | `x = 20` (tildel) |
| `the total is a plus b` | `total = a + b` |
| `change x by 5` | `x += 5` |

### Regnestykke (ord-former af operatorerne)

| Ord | Symbol |
|---|---|
| `plus` | `+` |
| `minus` | `-` |
| `times` / `multiplied by` | `*` |
| `divided by` | `/` |
| `the remainder of a divided by b` / `a mod b` | `%` |
| `to the power of` | `**` |
| `is greater than` | `>` |
| `is less than` | `<` |
| `is at least` | `>=` |
| `is at most` | `<=` |
| `is equal to` / `is the same as` / `is` | `==` |
| `is not equal to` / `is not` | `!=` |
| `and` / `or` / `not` | `&&` `\|\|` `!` |
| `contains` / `is in` / `is not in` | membership |

### Betingelser

```text
if alder is at least 18 then say "Voksen" otherwise say "Barn"     # én linje

if health is less than 10 then
    say "Advarsel!"
otherwise if health is less than 50 then
    say "Lav på liv"
otherwise
    say "Alt er godt"
done
```

`otherwise if` = else if. `unless C then ... done` = `if not C`.

### Gentagelser

| Naturlig | Betydning |
|---|---|
| `repeat 10 times ... done` | fast antal |
| `repeat with i from 1 to 10 ... done` | tæller (`i` = 1..10) |
| `repeat forever ... done` | uendelig |
| `repeat until the guess is correct ... done` | betinget slut |
| `repeat while there are items left ... done` | while |
| `repeat for each fruit in fruits ... done` | gennem løbe collection |
| `stop the loop` | break |
| `skip this one` / `go to next turn` | continue |

### Funktioner

```text
to greet with name
    say "Hej {name}!"
done

greet with "Carl"                       # kald
the message is greet result with "Carl" # fang returværdi

to double with n
    give back n times 2                 # give back = return
done
```

Flere parametre: `to add with a and b` — kald: `add with 2 and 3`.

### Programstart og moduler

```text
when the program starts            # ≡ fn main()
    ...
done

use the http library               # ≡ import std.net.http
use math from the standard library # ≡ from std.math import *
use json as j                      # alias
```

Scripts behøver ikke `when the program starts` — top-level kode kører top-down.

### Collections

```text
create a list called fruits
add "apple" to fruits
remove the first item of fruits
say how many items are in fruits              # ≡ fruits.len()
say the first item of fruits                  # ≡ fruits[0]
say the last item of fruits                   # ≡ fruits[^1]
say item 2 of fruits                          # ≡ fruits[1]
does fruits contain "apple"? → if fruits contains "apple" then ...
sort fruits
shuffle fruits
create a list of the numbers from 1 to 100    # ≡ 1..100 som Array
```

Map:

```text
create a map called ages
set the age of "Carl" in ages to 30           # ages["Carl"] = 30
say the age of "Carl" in ages
```

### Tekst

```text
say the name in capital letters                # upper()
say the first letter of the word
join parts with ", "
split the line by ","
if the text starts with "Hej" then ...
replace every space in s with "_"
```

### Objekter

```text
a Player is a thing with
    a name
    a health of 100

    to damage with amount
        take amount from my health             # my = self
    done
done

the hero is a new Player with name set to "Rex"
damage the hero with 20                        # metodekald som sætning
say the health of the hero                     # feltaflæsning
take 10 from the health of the hero            # felttilskrivning
```

`my` = self inde i definitionen, `its` = ejers refereret udefra (`its name`). Arv: `a Dog is a kind of Animal with ...`. Traits/interfaces udskydes til kompakt form i v1.

### Fejlhåndtering

```text
try
    open the file at path
if it fails as problem                         # catch, problem = fejl-objektet
    say "Det virkede ikke: {problem}"
done

the data is the file contents or nothing       # Result/Optional coalescing
```

### Match (check)

```text
check the status
    when it is "ok"       say "Alt godt"
    when it is "warning"  say "Pas på"
    otherwise             say "Ukendt"
done
```

### Tid, venten, samtidighed

```text
wait 2 seconds
wait until the file exists
in the background                              # ≡ spawn/task
    download the report
done
at the same time do task-a and do task-b       # ≡ parallel
```

### Sandhed

`true false nothing` (= true/false/none). Tomheds-test skal være eksplicit: `if xs has no items then ...` ≡ `xs.len() == 0` — samme regel som kernesproget (ingen implicit truthiness).

## 3. Frase-grammatik (skitse)

```ebnf
sentence    = command , "."? ;
command     = say_cmd | ask_cmd | set_cmd | change_cmd | if_cmd | repeat_cmd
            | to_def | check_cmd | try_cmd | create_cmd | use_cmd | wait_cmd
            | when_block | background | call_sentence ;

call_sentence = verb , [ "with" , arg , { "and" , arg } ] ;
arg         = expression ;
expression  = term , { arith_word , term } ;
arith_word  = "plus" | "minus" | "times" | "divided by"
            | "to the power of" | symbol_op ;          (* symboler tilladt *)
comparison  = [ expr ] , "is" , ["not"] , (
                "greater than" | "less than" | "at least" | "at most"
              | "equal to" | "the same as" | "a number" | "nothing"
              | literal ) ;
block       = NEWLINE , { sentence } , "done"
            | ":" , sentence ;                          (* én-linjes *)
noise_word  = "the" | "a" | "an" | "of" | "value" ;     (* ignoreres *)
```

Regler mod tvetydighed:

1. **Fast verbum først:** hver sætning starter med et reserveret verbum (`say ask set add take if unless repeat to check try use create wait when in at stop skip go return give`). Variabel-navne må derfor ikke kollidere med verbene (compiler-fejl med forslag).
2. **`is` har to roller**, adskilt af kontekst: `X is <udtryk>` = sammenligning/deklaration; `set X to V` = tildeling. Deklaration med regnestykke: `the total is a plus b`.
3. **Ejeforhold altid med `of`:** `the health of the hero` — aldrig bare to navne ved siden af hinanden.
4. Symbol-operatorer (`+ > ==`) accepteres overalt som stenografi for ord-formerne.

## 4. Desugaring (Natural → kerne-AST)

Natural-laget er en ren syntaktisk transformation **før** parsing-slut: tokens → frase-match → de normale AST-noder. Derfor får man automatisk: type-inference, ARC, alle stdlib-API'er, LSP, formatter — intet duplikeres.

```text
say X                    →  print(X)
ask Q and remember as N  →  N = stdin.line(Q)
set X to V               →  X = V
add V to X               →  X += V          (take V from X → -=)
repeat N times           →  for _ in 0..N
repeat until C           →  while not C
repeat for each I in XS  →  for I in XS
stop the loop            →  break
to F with A ... done     →  fn F(A) { }
give back V              →  return V
when the program starts  →  fn main()
a T is a thing with F    →  class T { F }
my X / its X             →  self.X
nothing                  →  none
how many items are in X  →  X.len()
the first item of X      →  X[0]        the last item of X → X[^1]
X is a number            →  X.to_int().ok (String-kontekst) / X is Numeric
in the background        →  spawn { }
wait N seconds           →  async.sleep(seconds(N))
```

## 5. Eksempler

### Hello, world

```text
when the program starts
    say "Hej, verden!"
done
```

### Lommeregner

```text
when the program starts
    a is ask "Første tal: "
    b is ask "Andet tal: "

    if a is a number and b is a number then
        say "Summen er [the number value of a plus the number value of b]"
    otherwise
        say "Skriv venligst tal."
    done
done
```

(`[...]` = interpolation-alternativ til `{...}` i natural-mode; begge gyldige.)

### Gøreliste

```text
use the standard library

things is an empty list

repeat forever
    command is ask "(tilføj/vis/færdig) > "

    check the command
        when it starts with "tilføj"
            add everything after "tilføj " in command to things
            say "Der er nu {how many items are in things} ting på listen."
        when it is "vis"
            repeat for each thing in things
                say "- {thing}"
            done
        when it is "færdig"
            stop the loop
        otherwise
            say "Prøv: tilføj / vis / færdig"
    done
done

say "Farvel!"
```

### Samtidighed

```text
when the program starts
    at the same time do count-sheep and do boil-water

to count-sheep
    repeat with i from 1 to 3
        wait 1 second
        say "{i} får..."
    done
done
```

## 6. Tooling-integration

- **Autocomplete skriver sætningerne færdige**: efter `rep` → `repeat 10 times … done` / `repeat for each … in … done`.
- **"Oversæt"-kommando:** `nova speak fil.nova` viser den kompakte form; `nova natural fil.nova` viser natural-formen. Formatteren kan konvertere mellem de to (samme AST).
- **Undervisnings-profil:** `project.nova: syntax = "natural-only"` slår shorthand fra (skoler/beginners).
