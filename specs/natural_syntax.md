# Nova Natural Syntax ("Nova Natural")

**Goal:** you should almost be able to *say* your idea to another human, write it down
word for word — and have an app.

```text
Idea spoken aloud:
    "If the player's health is under 10, say a warning. Otherwise say all is good."

Nova Natural:
    if the health of the player is less than 10 then
        say "Warning!"
    otherwise
        say "All is good."
    done
```

## 1. Principles

1. **Controlled English — not free-form NLP.** The language is still a formal grammar
   with fixed phrases. What looks like "English" is a fixed vocabulary of ~120 words in
   fixed patterns. The parser is deterministic; no AI interpretation.
2. **One AST, two skins.** Nova Natural is the primary surface. The compact symbol
   syntax (`{}`, `=>`, `+`, `>`, `print`) remains valid as expert stenography. Both forms
   compile to **identical AST** — mix freely.
3. **Blocks close with `done`.** Not indentation-sensitive (the formatter recommends
   indentation; the parser does not require it). Unambiguous and copy-paste safe.
   Single-line forms use `then`/`:` without `done`.
4. **Noise words allowed:** `the a an of it that value are is` can be used where they
   sound natural: `the length of xs` ≡ `length of xs`. The parser ignores them.
5. **Errors suggest:** typing `display x` makes the compiler reply *"Did you mean
   `say x`?"* (Levenshtein over the phrase vocabulary).

## 2. Command vocabulary (full table)

### Output and input

| Natural | Compact | Meaning |
|---|---|---|
| `say "hi"` | `print("hi")` | print + newline |
| `write "hi"` | `print(..., newline = false)` | print without newline |
| `ask "Name?" and remember it as name` | `name = stdin.line("Name?")` | ask and store the answer |
| `say x and y` | `print(x, y)` | multiple values |

### Variables

| Natural | Compact |
|---|---|
| `x is 10` | `x = 10` (create) |
| `set x to 20` | `x = 20` (assign) |
| `the total is a plus b` | `total = a + b` |
| `change x by 5` | `x += 5` |

### Arithmetic (word forms of the operators)

| Word | Symbol |
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

### Conditionals

```text
if age is at least 18 then say "Adult" otherwise say "Child"       # one line

if health is less than 10 then
    say "Warning!"
otherwise if health is less than 50 then
    say "Low on health"
otherwise
    say "All is good"
done
```

`otherwise if` = else if. `unless C then ... done` = `if not C`.

### Repetition

| Natural | Meaning |
|---|---|
| `repeat 10 times ... done` | fixed count |
| `repeat with i from 1 to 10 ... done` | counting (`i` = 1..10) |
| `repeat forever ... done` | infinite |
| `repeat until the guess is correct ... done` | conditional end |
| `repeat while there are items left ... done` | while |
| `repeat for each fruit in fruits ... done` | iterate a collection |
| `stop the loop` | break |
| `skip this one` / `go to next turn` | continue |

### Functions

```text
to greet with name
    say "Hello {name}!"
done

greet with "Carl"                       # call
the message is greet result with "Carl" # capture the return value

to double with n
    give back n times 2                 # give back = return
done
```

Multiple parameters: `to add with a and b` — call: `add with 2 and 3`.

### Program start and modules

```text
when the program starts            # ≡ fn main()
    ...
done

use the http library               # ≡ import std.net.http
use math from the standard library # ≡ from std.math import *
use json as j                      # alias
```

Scripts do not need `when the program starts` — top-level code runs top-down.

Bootstrap note (C05): file-based modules are `the tools-module in "tools.nova"`
(see module_system §0); the `use the X library` form binds stdlib namespaces (B03).

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
create a list of the numbers from 1 to 100    # ≡ 1..100 as Array
```

Map:

```text
create a map called ages
set the age of "Carl" in ages to 30           # ages["Carl"] = 30
say the age of "Carl" in ages
```

### Text

```text
say the name in capital letters                # upper()
say the first letter of the word
join parts with ", "
split the line by ","
if the text starts with "Hello" then ...
replace every space in s with "_"
```

### Objects

```text
a Player is a thing with
    a name
    a health of 100

    to damage with amount
        take amount from my health             # my = self
    done
done

the hero is a new Player with name set to "Rex"
damage the hero with 20                        # method call as a sentence
say the health of the hero                     # field read
take 10 from the health of the hero            # field assignment
```

`my` = self inside the definition, `its` = the owner referenced outside (`its name`).
Inheritance: `a Dog is a kind of Animal with ...`. Traits/interfaces deferred to the
compact form in v1.

### Error handling

```text
try
    open the file at path
if it fails as problem                         # catch, problem = the error object
    say "It did not work: {problem}"
done

the data is the file contents or nothing       # Result/Optional coalescing
```

### Match (check)

```text
check the status
    when it is "ok"       say "All good"
    when it is "warning"  say "Careful"
    otherwise             say "Unknown"
done
```

### Time, waiting, concurrency

```text
wait 2 seconds
wait until the file exists
in the background                              # ≡ spawn/task
    download the report
done
at the same time do task-a and do task-b       # ≡ parallel
```

### Truth

`true false nothing` (= true/false/none). Emptiness tests must be explicit:
`if xs has no items then ...` ≡ `xs.len() == 0` — same rule as the kernel language
(no implicit truthiness).

## 3. Phrase grammar (sketch)

```ebnf
sentence    = command , "."? ;
command     = say_cmd | ask_cmd | set_cmd | change_cmd | if_cmd | repeat_cmd
            | to_def | check_cmd | try_cmd | create_cmd | use_cmd | wait_cmd
            | when_block | background | call_sentence ;

call_sentence = verb , [ "with" , arg , { "and" , arg } ] ;
arg         = expression ;
expression  = term , { arith_word , term } ;
arith_word  = "plus" | "minus" | "times" | "divided by"
            | "to the power of" | symbol_op ;          (* symbols allowed *)
comparison  = [ expr ] , "is" , ["not"] , (
                "greater than" | "less than" | "at least" | "at most"
              | "equal to" | "the same as" | "a number" | "nothing"
              | literal ) ;
block       = NEWLINE , { sentence } , "done"
            | ":" , sentence ;                          (* one-line *)
noise_word  = "the" | "a" | "an" | "of" | "value" ;     (* ignored *)
```

Rules against ambiguity:

1. **Fixed verb first:** every sentence starts with a reserved verb (`say ask set add
   take if unless repeat to check try use create wait when in at stop skip go return
   give`). Variable names must therefore not collide with the verbs (compiler error
   with suggestion).
2. **`is` has two roles**, separated by context: `X is <expression>` =
   comparison/declaration; `set X to V` = assignment. Declaration with arithmetic:
   `the total is a plus b`.
3. **Ownership always with `of`:** `the health of the hero` — never just two names side
   by side.
4. Symbol operators (`+ > ==`) accepted everywhere as stenography for the word forms.

## 4. Desugaring (Natural → kernel AST)

The Natural layer is a pure syntactic transformation **before** parsing finishes:
tokens → phrase match → the ordinary AST nodes. You therefore get automatically:
type inference, ARC, every stdlib API, LSP, formatter — nothing duplicated.

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
X is a number            →  X.to_int().ok (String context) / X is Numeric
in the background        →  spawn { }
wait N seconds           →  async.sleep(seconds(N))
```

## 5. Examples

### Hello, world

```text
when the program starts
    say "Hello, world!"
done
```

### Calculator

```text
when the program starts
    a is ask "First number: "
    b is ask "Second number: "

    if a is a number and b is a number then
        say "The sum is [the number value of a plus the number value of b]"
    otherwise
        say "Please enter numbers."
    done
done
```

(`[...]` = interpolation alternative to `{...}` in natural mode; both valid.)

### Todo list

```text
use the standard library

things is an empty list

repeat forever
    command is ask "(add/show/done) > "

    check the command
        when it starts with "add"
            add everything after "add " in command to things
            say "There are now {how many items are in things} things on the list."
        when it is "show"
            repeat for each thing in things
                say "- {thing}"
            done
        when it is "done"
            stop the loop
        otherwise
            say "Try: add / show / done"
    done
done

say "Bye!"
```

### Concurrency

```text
when the program starts
    at the same time do count-sheep and do boil-water

to count-sheep
    repeat with i from 1 to 3
        wait 1 second
        say "{i} sheep..."
    done
done
```

## 6. Tooling integration

- **Autocomplete completes the sentences**: after `rep` → `repeat 10 times … done` /
  `repeat for each … in … done`.
- **A "translate" command:** `nova speak file.nova` shows the compact form;
  `nova natural file.nova` shows the natural form. The formatter can convert between
  them (same AST).
- **Teaching profile:** `project.nova: syntax = "natural-only"` disables shorthand
  (schools/beginners).
