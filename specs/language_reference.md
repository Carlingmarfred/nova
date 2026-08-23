# Nova Language Reference (full)

Status: spec v0.9. All code below is normative example material.

> **Syntax modes:** This reference uses the **compact shorthand form** (`{}` blocks, `=>`, symbol operators). The primary surface is **Nova Natural** (see [natural_syntax.md](natural_syntax.md)) — plain English words: `say x` ≡ `print(x)`, `set x to 10` ≡ `x = 10`, `repeat until c ... done` ≡ `while !c { }`, `to greet with name ... done` ≡ `fn greet(name) { }`. Both forms produce the **identical AST** and mix freely in the same file.

---

## 1. Variables and bindings

```text
x = 10              # mutable binding, type inferred → i32
let y = 3.14        # immutable, → f64
var z: i64 = 10     # 'var' is a synonym for plain assignment (explicit style)
x = 20              # ok
y = 1.0             # ERROR: immutable
```

Rules:

- `name = expression` creates a **mutable** binding with inference (Python habit).
- `let` = immutable. `let mut x` is NOT syntax — `mut` does not exist; use plain.
- Type annotation: `name: Type = value`. Annotation without initializer allowed only for class fields and parameters.
- Scope: block scope, shadowing allowed in inner scopes (`let x = x + 1` is fine).
- Module-level constants: `const PI = 3.14159` (compile-time evaluated).
- Top-level code allowed in scripts (`main.nova` runs top-down); libraries use `fn main()`.

## 2. Data types

### 2.1 Primitives

| Category | Types | Default literal |
|---|---|---|
| Signed integers | `i8 i16 i32 i64 i128 isize` | `42` → `i32` |
| Unsigned integers | `u8 u16 u32 u64 u128 usize` | `42u8` etc. |
| BigInt (arbitrary precision) | `BigInt` | `99999999999999999999999999n` |
| Float | `f32 f64` | `3.14` → `f64`; `2.5f32` |
| Decimal/Rational | `Decimal`, `Rational` | `Decimal("0.1")` |
| Complex | `Complex<f64>` | `3 + 4i` |
| Other | `bool`, `char`, `String`, `()` (unit) | |

Overflow: debug builds panic; release wraps. Opt-in strictness: `@checked fn ...`.

Conversion: explicit with `as` (numeric), `to::<T>()` (fallible), `.parse::<i32>()?`.

```text
b = 255u8
big = b as u32            # always ok
lossy = 300 as u8         # allowed but linter warning (wraps)
safe = 300.to::<u8>()     # Result<u8> — Err on overflow
n = "42".parse::<i32>()?
```

### 2.2 Collections

| Type | Description | Python counterpart |
|---|---|---|
| `Array<T>` | growable dynamic array | `list` |
| `(A, B)` | tuple, heterogeneous, fixed size | `tuple` |
| `Map<K,V>` | hash-map | `dict` |
| `Set<T>` | hash-set | `set` |
| `SortedMap<K,V>` / `SortedSet<T>` | ordered (B-tree) | — |
| `Deque<T>` | double-ended queue | `collections.deque` |
| `Heap<T>` | priority queue | `heapq` |
| `[T; N]` | fixed-size array (stack) | — |
| `Range` | `a..b` (exclusive), `a..=b` (inclusive) | `range` |
| `Iterator<T>` | lazily chainable | iterator protocol |
| `Bytes`, `StringBuilder` | byte-array / string-builder | `bytes`, io.StringIO |

Literals:

```text
xs   = [1, 2, 3]                    # Array<i32>
pair = (1, "hi")                    # (i32, String)
m    = {"a": 1, "b": 2}             # Map<String,i32>
s    = {1, 2, 3}                    # Set<i32>
fxd  = [1, 2, 3] as [i32; 3]        # stack-allocated
rng  = 0..10                        # Range<i32>, exclusive
```

### 2.3 Optional and Result

```text
Optional<T> ≡ T?          # values: some(v) | none
Result<T,E>               # values: Ok(v) | Err(e)
```

```text
maybe: i32? = none
r = File.read("x.txt")    # Result<String, IoError>
```

See error_handling.md for `?`, `??`, `?.` etc.

### 2.4 dynamic

```text
d: dynamic = get_json()
d.name                       # runtime lookup, returns dynamic
d.items[0].price as f64      # explicit conversion
```

Full rules in type_system.md §7.

## 3. Operators (complete table)

Precedence low to high. All left-associative except where noted.

| Level | Operators | Note |
|---|---|---|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` `//=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `??=` | assignment (right-assoc.) |
| 2 | `\|\|` `or` | short-circuits |
| 3 | `&&` `and` | short-circuits |
| 4 | `!in` `in` `is not` `is` | membership / type test |
| 5 | `==` `!=` `<` `<=` `>` `>=` `<=>` | comparison, non-chainable |
| 6 | `..` `..=` `..<` | range (non-assoc.) |
| 7 | `\|` `^` | bitwise |
| 8 | `&` | bitwise |
| 9 | `<<` `>>` | shift |
| 10 | `+` `-` | |
| 11 | `*` `/` `%` `//` | `//` = floor division |
| 12 | `**` | power (right-assoc.) |
| 13 | unary `-` `+` `!` `not` `~` `*` (deref) `&` (addr-of, unsafe) | |
| 14 | postfix `?` `?.` `!` `[]` `()` `.` `?.` `as` `::` | |

Extras:

- `??` — nil-coalescing: `a ?? b` = `if a == none then b`.
- `?.` — optional chaining: `obj?.field?.method()` yields `none` at the first `none`.
- `is` — type test: `x is String`, `x is Array<i32>`.
- `as` — cast/conversion.
- `in` — membership: `x in xs`, `key in map`.
- `=>` — lambda arrow and match arms.
- No `++`/`--` — use `x += 1`.

Overloading: operators are overloaded via traits (`Add`, `Sub`, `Mul`, `Div`, `Index`, `Compare`, `Equals`, `Iterate`, `Call`, ...) — see §10.

## 4. Control flow

### 4.1 if / else if / else

```text
if x > 10 {
    print("big")
} else if x > 5 {
    print("medium")
} else {
    print("small")
}
```

If is an **expression**:

```text
category = if x > 10 { "big" } else { "small" }
```

### 4.2 while / loop / for-in

```text
while condition { ... }
loop { ... }                          # infinite; ends via break/return

for x in xs { print(x) }              # anything Iterable
for i in 0..xs.len() { ... }          # index loop
for (i, x) in xs.enumerate() { ... }  # index + value
for (k, v) in map { ... }             # Map iterates (K,V)-pairs
```

Labels:

```text
outer: for i in 0..10 {
    for j in 0..10 {
        if i * j > 50 { continue outer }
        if i + j > 99 { break outer }
    }
}
```

`for` loops are sugar over the `Iterator` trait's `next()`.

### 4.3 match (exhaustive)

```text
match value {
    0                => "zero"
    1 | 2 | 3        => "small number"
    n if n % 2 == 0  => "even"
    n                => "odd: {n}"
}

match point {
    Point(0, 0)       => "origin"
    Point(x, 0)       => "x-axis: {x}"
    Point(_, y) where y > 0 => "above"
    _                 => "others"
}

match opt {
    some(v) => v
    none    => 0
}
```

Patterns (fully composable): literal, wildcard `_`, binding, tuple, struct `{name, age}`, enum `Variant(pats)`, range `1..=10`, array `[first, ...rest]`, slice `[a, b, ..]`, type-test `x is T`, or-pattern `a | b`, guard `where cond`. The compiler verifies **exhaustiveness** and dead branches.

Match is an expression and must produce the same type in all arms (or unit).

### 4.4 try / catch (panic handlers)

```text
try {
    risky()
} catch e: PanicError {
    log(e.message)
} finally {
    cleanup()
}
```

Used only for exceptional cases — never control flow (see error_handling.md).

### 4.5 use (context management, RAII)

Python `with` ↔ Nova `use`:

```text
use f = File.open("data.txt") {
    contents = f.read_all()
}                                   # f.close() is called deterministically here

use lock = mutex.acquire() { ... }  # lock released at block end
use (a, b) = acquire_two() { ... }  # destructuring use
```

Requirement: the type implements trait `Disposable { fn dispose(self) }`.

## 5. Functions

```text
fn add(a: i32, b: i32) -> i32 {
    a + b                            # last expression = return value
}

fn greet(name = "world", times: i32 = 1) {      # default values
    ("Hello {name}! " * times).trim()
}

greet(times = 3)                     # named arguments
```

### 5.1 Variadic parameters

```text
fn sum(...nums: i32) -> i32 {        # positional variadic
    nums.iter().sum()                # nums is Array<i32>
}

fn config(**opts: dynamic) {         # keyword-collect → Map<String, dynamic>
    opts.timeout ?? 30
}

sum(1, 2, 3)
config(timeout = 5, retries = 2)     # named args collected into opts
```

Spread at call site: `sum(...[1, 2, 3])`.

### 5.2 Lambdas and closures

```text
square = x => x * x
add    = (a, b) => a + b
body   => { let t = prep(); compute(t) }

factor = 3
scale  = x => x * factor             # closure captures environment (by capture)
```

Capture rule: default **by reference** with ARC; the compiler copies automatically when the lifetime requires it (escape). Explicit: `[x, &y] => ...` (capture by value / by ref).

Function types: `fn(i32, i32) -> i32`, nullable: `(fn(i32) -> bool)?`.

### 5.3 Overloads and generic functions

```text
fn parse(s: String) -> i32 { ... }       # overloading allowed on parameter types
fn parse(s: String) -> f64 { ... }

fn max<T: Comparable>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn first<T>(xs: [T]) -> T? { xs.first() }   # [T] = Array<T> in generic context
```

Generics are monomorphized (native backend); the VM uses type-erased + caches. Bounds via traits. See type_system.md.

### 5.4 Recursion and tail calls

Tail-call optimization guaranteed for self-recursion marked `@tailrec` (compiler error if not in tail position).

```text
@tailrec
fn count(n: i32, acc: i32 = 0) -> i32 {
    if n == 0 { acc } else { count(n - 1, acc + n) }
}
```

### 5.5 Doc comments

```text
/// Returns the area of a circle.
///
/// # Example
/// area = circle_area(2.0)  // ≈ 12.566
fn circle_area(r: f64) -> f64 { PI * r ** 2 }
```

Doctests run via `nova test --doc`.

## 6. Collection operations (stdlib-core)

All methods returning a new collection are lazy where possible (iterator-based):

```text
xs = [5, 3, 8, 1]

# Transform
xs.map(x => x * 2)                 # [10, 6, 16, 2]
xs.filter(x => x > 2)              # [5, 3, 8]
xs.rev()                           # [1, 8, 3, 5]
xs.sorted()                        # [1, 3, 5, 8]
xs.sorted(by = (a, b) => b <=> a)  # descending
xs.zip([9, 8, 7])                  # [(5,9),(3,8),(8,7)]
xs.chunked(2)                      # [[5,3],[8,1]]
xs.windowed(2)                     # [[5,3],[3,8],[8,1]]
xs.flat_map(x => [x, x])           # [5,5,3,3,8,8,1,1]

# Reduce / query
xs.sum()  xs.product()  xs.min()  xs.max()
xs.count(x => x > 2)  xs.any(x => x > 7)  xs.all(x => x > 0)
xs.fold(0, (acc, x) => acc + x)
xs.reduce((a, b) => a + b)
xs.find(x => x > 4)                # some(5) | none
xs.position(x => x == 8)           # Index? = some(2)
xs.group_by(x => x % 2)            # Map<Bool, Array>

# Index/slice (negative index ^1 = last)
xs[0]  xs[^1]  xs[1..3]  xs[..2]  xs[2..]  xs[0.. by 2]
xs[1:3]                            # Python-style sugar for 1..<3

# Mutation
xs.push(4)  xs.pop()  xs.insert(0, 9)  xs.remove_at(2)
xs.extend([7, 7])  xs.clear()  xs.sort()  xs.reverse()

# Membership/instance
3 in xs  xs.index_of(8)  xs.unique()  xs.contains_all([1, 3])
```

Map/Set:

```text
m = {"a": 1}
m["b"] = 2            m.get("c")           # none, not an exception
m.get_or_insert("c", 0)
"b" in m              m.keys()  m.values()  m.items()
{k: v * 2 for (k, v) in m}                 # dict comprehension
{x % 3 for x in 0..30}                     # set comprehension
m.merge({"z": 9})     m | {"w": 0}         # union operators
```

Comprehensions (general form):

```text
[expr for x in iter if cond if cond2]
[k: v for (k, v) in pairs]
{elem for x in iter}
```

## 7. Strings

```text
s = "World"
name = "Nova"
msg = "Hi {name}, {1 + 1}"           # interpolation: Hi Nova, 2
pi_msg = "π ≈ {PI:.3}"               # format spec (like Python's :.3f)
raw = r"C:\path\without\escapes"
multi = """
    multiple lines
"""
bytes_s = b"ABC"                     # Bytes
ch = 'A'                             # char (Unicode scalar)

# Methods (excerpt — full list in standard_library.md §core.string)
s.lower()  s.upper()  s.trim()  s.strip(" ")
s.split(",")  s.rsplit(",", max = 1)  s.split_lines()
"-".join(["a", "b"])  s.replace("e", "E")  s.remove_prefix("Hi")
s.starts_with("W")  s.ends_with("d")  s.find("r")  s.count("e")
s.repeat(2)  s.pad_left(10)  s.center(20, '*')
s.chars()  s.bytes()  s.code_points()  s.graphemes()   # iterators
s.to::<i32>()  "3.14".to::<f64>()
s <=> "abc"                          # Unicode-correct collation

# Slice like collections
s[0..3]  s[^5..]  s[::-1]            # backwards via step -1
```

Strings are UTF-8; indexing is a **byte index** and O(1) access is unsafe per char — therefore `s.chars()[i]` for positional access (the linter flags `s[i]` on String).

## 8. Classes, structs, enums, traits

### 8.1 struct (value type, data)

```text
struct Vec3 {
    x: f32
    y: f32
    z: f32
}

v = Vec3(x = 1, y = 2, z = 3)        # named init
v.x += 10
w = Vec3(..v, z = 99)                # functional update
dist = (v.x ** 2 + v.y ** 2 + v.z ** 2).sqrt()
```

Structs have value semantics (copy or move), no inheritance. Fields can have defaults.

### 8.2 class (reference type, inheritance)

```text
class Animal {
    name: String

    fn init(name: String) {
        self.name = name
    }

    virtual fn sound() -> String { "..." }
}

class Dog : Animal {
    breed: String

    fn init(name: String, breed: String) {
        super.init(name)
        self.breed = breed
    }

    override fn sound() -> String { "Woof" }
}

d = Dog("Rex", "labrador")
d.sound()
```

- Single inheritance (`:`), interfaces via traits.
- `virtual`/`override` are **required words** — no accidental polymorphism.
- Fields: `pub`, `priv` (default), `protected`. Properties:

```text
class Account {
    priv balance_: f64 = 0

    balance: f64 {
        get { self.balance_ }
        set(v) {
            if v < 0 { throw Error("negative balance") }
            self.balance_ = v
        }
    }
}
```

- Static members: `static fn create()`.
- Destructors: `fn deinit()` (called deterministically under ARC).
- Auto-generation: `@derive(Equals, Compare, Hashable, Printable, Serializable)`.

### 8.3 Data classes

```text
@dataclass
class Person {
    name: String
    age: i32 = 0
}
# gives for free: init with named args, Equals, Hashable, Printable,
# copy-with, Serializable (see metaprogramming.md)
```

### 8.4 enums (tagged unions)

```text
enum Shape {
    Circle(r: f64)
    Rect(w: f64, h: f64)
    Triangle(a: f64, b: f64, c: f64)

    fn area(self) -> f64 {
        match self {
            Circle(r) => PI * r ** 2
            Rect(w, h) => w * h
            Triangle(a, b, c) => heron(a, b, c)
        }
    }
}

enum Status { Active | Inactive(reason: String?) }
```

Enums can have methods, shared fields and generics. Simple enums: `enum Color { Red | Green | Blue }`.

### 8.5 traits (interfaces + mixins)

```text
trait Drawable {
    fn draw(self, canvas: Canvas)       # required method

    fn draw_twice(self, canvas: Canvas) {   # default implementation
        self.draw(canvas)
        self.draw(canvas)
    }
}

impl Drawable for Circle {
    fn draw(self, canvas: Canvas) { canvas.circle(self.center, self.r) }
}
```

- Trait objects (dynamic dispatch): `dyn Drawable`.
- Generic bounds: `fn render<T: Drawable>(items: Array<T>)`.
- Traits can require associated types/constants and have blanket impls (std traits).
- Operator overloading and `Iterable`, `Index`, `Comparable` etc. are ordinary traits.

### 8.6 Extension methods

```text
extend String {
    fn shout(self) -> String { self.upper() + "!" }
}

"hey".shout()
```

## 9. Modules

```text
mod math_utils {
    pub fn helper() {}
    fn internal() {}                 # private to the module
}

import math_utils
import std.io.File
import std.json as json
from std.math import sqrt, PI

export                              # makes all of the file's pub symbols public API
```

Details in module_system.md.

## 10. Operator overloading (traits table)

| Expression | Trait to implement |
|---|---|
| `a + b` | `Add` (`fn add`) |
| `a - b` | `Sub` |
| `a * b` | `Mul` |
| `a / b` | `Div` |
| `a // b` | `FloorDiv` |
| `a % b` | `Mod` |
| `a ** b` | `Pow` |
| `-a` | `Neg` |
| `~a` | `Invert` |
| `a & b` `\| ^ << >>` | `BitAnd` `BitOr` `BitXor` `Shl` `Shr` |
| `a == b` | `Equals` |
| `a < b` etc. | `Compare` (`fn cmp -> Ordering`) |
| `xs[i]` / `xs[i] = v` | `Index` / `IndexSet` |
| `xs[a..b]` | `Slice` |
| `for x in obj` | `Iterable` (`fn iterator`) |
| `obj(...)` | `Callable` (`fn call`) |
| `"x" * 3` | `Mul` with asymmetric types allowed |
| `f"{obj}"` | `Printable` (`fn format(fmt) -> String`) |
| `truthy(obj)` / `if obj` | `Truthiness` (default: everything except `false`/`none`/`0`/`""` is true? NO — only `bool` and `bool?` are conditions; anything else is a compile error) |

**Deliberate deviation from Python:** conditions require a real `bool`. The linter may relax locally (`#nova allow truthiness`). This removes an entire class of `=` vs `==`/emptiness bugs.

## 11. Iterators and generators

```text
fn fibonacci() -> Iterator<i32> {
    var a = 0
    var b = 1
    loop {
        yield a
        (a, b) = (b, a + b)
    }
}

fib = fibonacci().take(10)          # lazy
even_fibs = fib.filter(x => x % 2 == 0)
```

`yield` is rewritten by the compiler into a state machine (like async). Generators are just functions returning `Iterator<T>`. Iterator chains are zero-cost after inlining (monomorphized).

Iterator adapters (full list in stdlib): `map filter take skip take_while skip_while enumerate zip chain interleave flat_map scan fuse peekable chunked windowed step_by rev sorted unique group_by product permutations combinations`.

Terminal operations: `collect to_array to_set to_map sum min max count any all find fold reduce for_each join partition_by`.

## 12. Error handling (brief — full version in error_handling.md)

```text
fn read_config(path: String) -> Result<Config, ConfigError> {
    text = File.read(path)?                  # propagates
    parsed = json.parse(text)?
    Config.from_json(parsed)
}

port = env.get("PORT").and_then(v => v.parse::<i32>().ok()) ?? 8080

assert(xs.len() > 0, "list must not be empty")   # panic in debug, no-op in release
require(input != null, "input is required")      # always-active guard
```

## 13. Async and parallel (brief — full in concurrency.md)

```text
async fn fetch(url: String) -> Bytes {
    resp = await http.get(url)
    resp.body
}

async fn fetch_all(urls: Array<String>) {
    results = await gather(urls.map(fetch))     # concurrent
    ...
}

parallel {
    a = calculate_a()
    b = calculate_b()
}                                              # a and b run concurrently
```

## 14. Reflection

```text
t = typeof(Person)
print(t.name)                        # "Person"
for f in t.fields {
    print("{f.name}: {f.type}")
}

p = Person(name = "Carl", age = 30)
v = p["name"]                        # reflection-index (dynamic)
q = t.construct({name: "Anna"})      # dynamic construction

if obj is Drawable { obj.draw(c) }   # type test + cast
d = obj as dyn Drawable              # trait-object cast (Result on failure)
```

Runtime metadata is included in `--runtime full`; in minimal it is stripped (reflection calls = compile error).

## 15. Attributes (decorators/metadata)

```text
@test
fn addition_works() {
    expect(add(1, 2) == 3)
}

@deprecated("use new_api()")
fn old_api() {}

@benchmark(warmup = 100)
fn matrix_bench() {}

@generate_serialization              # macro (metaprogramming.md)
struct Order { id: Uuid, total: f64 }

@inline @cold @simd @checked @pure @noinline
@gpu(block = (256, 1, 1)) @thread_local @volatile
```

Attributes with arguments can be arbitrary compile-time expressions. User-defined attribute macros transform the AST (see metaprogramming.md).

## 16. Compile-time evaluation

```text
const TABLE = [x ** 2 for x in 0..256]      # computed at compile time

@compile
fn gen_primes(limit: i32) -> Array<i32> { ... }

const PRIMES = gen_primes(100)
```

Anything `@pure`-compatible can run at compile time: loops, match, generics — not IO/threads/dynamic.

## 17. Unsafe and raw memory (brief — full in memory_model.md)

```text
unsafe {
    buf = malloc(n)
    defer { free(buf) }                  # defer: called at scope exit, LIFO
    ptr = &buf[0]
    *ptr = 42
}
```

`defer` also exists in safe code (scope-bound cleanup without a trait requirement).

## 18. Scripting and shebang

```text
#!/usr/bin/env nova
print("script!")
args = CLI.args()                        # command-line arguments
exit(CLI.exit_code)
```

`nova run script.nova` starts the VM directly (no link phase). REPL: `nova repl`.

## 19. Reserved keywords (complete)

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
_ (wildcard is not a keyword but a symbol)
```

`null` exists only in unsafe/FFI contexts (raw pointers).

## 20. Pipelines — DECIDED

```text
# Compact
contents |> split_lines() |> filter(l => l.len() > 0) |> sorted() |> join("\n") |> print()
```

```text
# Natural — reads as a sentence:
take the file contents
    then split it by lines
    then keep the ones that are not empty
    then sort them
    then say the result
done
```

Desugars to ordinary method chains (zero cost after inlining). `|>` sends the left value as the first argument to the right call. The Natural phrases (`then split it by`, `then keep the ones that`, `then turn every X into`) are fixed templates in the natural_syntax.md vocabulary.

## 21. Signals (reactive state) — DECIDED

```text
score = signal(0)
rank  = computed(() => "Level {score.value / 100}")
effect { print("{score.value} → {rank.value}") }     # re-runs on change
score.value += 60                                     # → automatic update
```

Natural:

```text
the score is a signal starting at 0
when the score changes
    say "{score} → {rank}"
done
add 50 to the score
```

Semantics:

- Pull-based, glitch-free topological invalidation (SolidJS/SwiftUI model): derived values recompute only when read, never in intermediate states.
- `computed` memoizes; dependency tracking is automatic (no dependency lists).
- `effect` runs after commit; exceptions in effects panic normally.
- GUI integration: nova-gui binds directly — `the label text binds to the rank`.
- Async sources: `stream.into_signal()` drives a signal from events/network.

## 22. Actors — DECIDED

Full semantics: concurrency.md §7. Short form:

```text
actor Counter {
    total: i64 = 0
    on add(n: i64)  { self.total += n }
    on get() -> i64 { self.total }
}

c = Counter()
c.send(.add(5))
print(c.request(.get()))
```

One message at a time per actor → no locks; fields touched only from own handlers.

## 23. Contracts — DECIDED

```text
fn withdraw(amount: f64) {
    requires(amount > 0)
    requires(amount <= self.balance)
    ensures(self.balance >= 0)

    self.balance -= amount
}
```

Natural: `requires amount is greater than 0` / `ensures my balance is at least 0`.

Rules:

- `requires` evaluated at entry, `ensures` at exit (with access to the return value via `result`).
- Checked in debug/tests; stripped in release. Profile: `contracts = "debug" | "always" | "never"`.
- Contract expressions must be `@pure`.
- The same contracts drive: fuzzing input generation (testkit), docs/hover display and the refinement verification (type_system §11).
