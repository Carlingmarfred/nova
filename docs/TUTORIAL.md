# Describe Your First App — Nova Tutorial

## What you'll build

A notes manager you run from the terminal:

```
nova run notes.nova add "buy milk"
nova run notes.nova list
nova run notes.nova search milk
nova run notes.nova delete 1
```

## Step 1 — Say hello

Create a file called `hello.nova`:

```text
when the program starts
    say "Hello, world!"
done
```

Run it:

```
python bootstrap/nova_cli.py run hello.nova
```

You just wrote your first Nova program. Every statement reads like English.
Blocks close with `done`.

## Step 2 — Variables and arithmetic

```text
x is 10
y is x plus 5
say "x = {x}, y = {y}"
```

Variables are declared with `is` and reassigned with `set ... to`.
Arithmetic uses words: `plus`, `minus`, `times`, `divided by`.

## Step 3 — Conditionals

```text
age = 20
if age is at least 18 then say "Adult" otherwise say "Minor"
```

Or multi-line:

```text
if health is less than 10 then
    say "Warning!"
otherwise if health is less than 50 then
    say "Low on health"
otherwise
    say "All is good"
done
```

Multi-line blocks close with `done`. Single-line needs no `done`.

## Step 4 — Loops

```text
repeat 3 times
    say "echo"
done

repeat with i from 1 to 5
    say "{i}..."
done

items is ["apple", "banana", "cherry"]
repeat for each fruit in items
    say "- {fruit}"
done
```

## Step 5 — Functions

```text
to greet with name
    say "Hello, {name}!"
done

greet with "World"
```

Functions that return values use `give back`:

```text
to double with n
    give back n times 2
done

say double(21)       # prints 42
```

## Step 6 — Lists and modules

```text
use the standard list library
use the standard text library
use the standard flow library

fruits = ["apple", "banana", "cherry"]
say the length of fruits          # 3
say the first item of fruits      # apple

sorted = list.sort(fruits)
say sorted                        # [apple, banana, cherry]

upper = text.upper("hello")
say upper                         # HELLO

doubled = flow.map(x => x plus 0, [1, 2, 3])
say doubled                       # [2, 4, 6]
```

## Step 7 — Files and JSON

```text
use the standard file library
use the standard json library

file.write("data.json", json.stringify(["milk", "bread"]))
loaded = json.parse(file.read("data.json"))
say "{the length of loaded} items loaded"
```

## Step 8 — Your app: Notes CLI

Now put it all together. Here's the complete notes manager from `examples/notes.nova`:

```text
use the standard cli library
use the standard json library
use the standard file library
use the standard text library
use the standard flow library

args = cli.args()
command = item 1 of args

if command is "add" then
    rest = flow.skip(1, args)
    note-body = text.join(rest, " ")
    ...
done
```

Read the full source at `examples/notes.nova` and run it yourself.

## Next steps

- Read [specs/natural_syntax.md](specs/natural_syntax.md) for the full syntax reference
- Try `examples/guessing_game.nova` and `examples/todo.nova`
- Run `python bootstrap/nova_cli.py repl` for interactive exploration
