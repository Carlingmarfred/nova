# Nova Sprogreference (fuld)

Status: spec v0.9. Alt kode herunder er normativt eksempel-materiale.

> **Syntax-modes:** Denne reference bruger den **kompakte shorthand-form** (`{}`-blokke, `=>`, symbol-operatorer). Den primære brugerflade er **Nova Natural** (se [natural_syntax.md](natural_syntax.md)) — almindelige engelske ord: `say x` ≡ `print(x)`, `set x to 10` ≡ `x = 10`, `repeat until c ... done` ≡ `while !c { }`, `to greet with name ... done` ≡ `fn greet(name) { }`. Begge former producerer **identisk AST** og kan blandes frit i samme fil.

---

## 1. Variabler og bindinger

```text
x = 10              # mutable binding, type infereret → i32
let y = 3.14        # immutable, → f64
var z: i64 = 10     # 'var' er synonym for plain assignment (eksplicit stil)
x = 20              # ok
y = 1.0             # FEJL: immutable
```

Regler:

- `navn = udtryk` opretter **mutable** binding med inference (Python-vane).
- `let` = immutable. `let mut x` er IKKE syntaks — `mut` findes ikke; brug plain.
- Type-annotation: `navn: Type = værdi`. Annotation uden initializer tilladt kun for klassefelter og parametre.
- Scope: block-scope, skygge tilladt i indre scope (`let x = x + 1` ok).
- Konstanter på modulniveau: `const PI = 3.14159` (compile-time evalueret).
- Top-level kode er tilladt i scripts (`main.nova` kører top-down); biblioteker bruger `fn main()`.

## 2. Datatyper

### 2.1 Primitiver

| Kategori | Typer | Default-literal |
|---|---|---|
| Heltal signed | `i8 i16 i32 i64 i128 isize` | `42` → `i32` |
| Heltal unsigned | `u8 u16 u32 u64 u128 usize` | `42u8` osv. |
| BigInt (arbitrary precision) | `BigInt` | `99999999999999999999999999n` |
| Float | `f32 f64` | `3.14` → `f64`; `2.5f32` |
| Decimal/Rational | `Decimal`, `Rational` | `Decimal("0.1")` |
| Complex | `Complex<f64>` | `3 + 4i` |
| Andre | `bool`, `char`, `String`, `() ` (unit) | |

Overflow: debug-builds paniker; release wrappes. Opt-in streng: `@checked fn ...`.

Konvertering: eksplicit med `as` (numerisk), `to::<T>()` (fallible), `.parse::<i32>()?`.

```text
b = 255u8
big = b as u32            # altid ok
lossy = 300 as u8         # tilladt men linter-advarsel (wraps)
safe = 300.to::<u8>()     # Result<u8> — Err ved overflow
n = "42".parse::<i32>()?
```

### 2.2 Collections

| Type | Beskrivelse | Python-modsvar |
|---|---|---|
| `Array<T>` | growable dynamisk array | `list` |
| `(A, B)` | tuple, heterogen, fixed size | `tuple` |
| `Map<K,V>` | hash-map | `dict` |
| `Set<T>` | hash-set | `set` |
| `SortedMap<K,V>` / `SortedSet<T>` | ordnede (B-træ) | — |
| `Deque<T>` | double-ended queue | `collections.deque` |
| `Heap<T>` | priority queue | `heapq` |
| `[T; N]` | fixed-size array (stack) | — |
| `Range` | `a..b` (eksklusiv), `a..=b` (inklusiv) | `range` |
| `Iterator<T>` | lazily chainable | iterator-protokol |
| `Bytes`, `StringBuilder` | byte-array / string-builder | `bytes`, io.StringIO |

Literals:

```text
xs   = [1, 2, 3]                    # Array<i32>
pair = (1, "hej")                   # (i32, String)
m    = {"a": 1, "b": 2}             # Map<String,i32>
s    = {1, 2, 3}                    # Set<i32>
fxd  = [1, 2, 3] as [i32; 3]        # stack-allokeret
rng  = 0..10                        # Range<i32>, eksklusiv
```

### 2.3 Optional og Result

```text
Optional<T> ≡ T?          # værdier: some(v) | none
Result<T,E>               # værdier: Ok(v) | Err(e)
```

```text
maybe: i32? = none
r = File.read("x.txt")    # Result<String, IoError>
```

Se error_handling.md for `?`, `??`, `?.` osv.

### 2.4 dynamic

```text
d: dynamic = get_json()
d.name                       # runtime lookup, returnerer dynamic
d.items[0].price as f64      # eksplicit konvertering
```

Fulde regler i type_system.md §7.

## 3. Operatorer (komplet tabel)

Præcedens fra lav til høj. Alle venstre-associerende undtagen hvor angivet.

| Niveau | Operatorer | Bemærkning |
|---|---|---|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` `//=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `??=` | assignment (højre-assoc.) |
| 2 | `\|\|` `or` | kortslutter |
| 3 | `&&` `and` | kortslutter |
| 4 | `!in` `in` `is not` `is` | membership / typetest |
| 5 | `==` `!=` `<` `<=` `>` `>=` `<=>` | sammenligning, ikke-kædbare |
| 6 | `..` `..=` `..<` | range (ikke-assoc.) |
| 7 | `\|` `^` | bitwise |
| 8 | `&` | bitwise |
| 9 | `<<` `>>` | shift |
| 10 | `+` `-` | |
| 11 | `*` `/` `%` `//` | `//` = floor division |
| 12 | `**` | potens (højre-assoc.) |
| 13 | unary `-` `+` `!` `not` `~` `*` (deref) `&` (addr-of, unsafe) | |
| 14 | postfix `?` `?.` `!` `[]` `()` `.` `?.` `as` `::` | |

Ekstra:

- `??` — nil-coalescing: `a ?? b` = `if a == none then b`.
- `?.` — optional chaining: `obj?.field?.method()` giver `none` ved første `none`.
- `is` — typetest: `x is String`, `x is Array<i32>`.
- `as` — cast/konvertering.
- `in` — membership: `x in xs`, `key in map`.
- `=>` — lambda-arrow og match-arme.
- Ingen `++`/`--` — brug `x += 1`.

Overloading: operatorer overloades via traits (`Add`, `Sub`, `Mul`, `Div`, `Index`, `Compare`, `Equals`, `Iterate`, `Call`, ...) — se §10.

## 4. Kontrolflow

### 4.1 if / else if / else

```text
if x > 10 {
    print("stor")
} else if x > 5 {
    print("mellem")
} else {
    print("lille")
}
```

If er et **udtryk**:

```text
kategori = if x > 10 { "stor" } else { "lille" }
```

### 4.2 while / loop / for-in

```text
while betingelse { ... }
loop { ... }                          # uendelig; afsluttes med break/return

for x in xs { print(x) }              # alle Iterable
for i in 0..xs.len() { ... }          # indeks-loop
for (i, x) in xs.enumerate() { ... }  # indeks + værdi
for (k, v) in map { ... }             # Map itererer (K,V)-par
```

Labels:

```text
ydre: for i in 0..10 {
    for j in 0..10 {
        if i * j > 50 { continue ydre }
        if i + j > 99 { break ydre }
    }
}
```

`for`-loops er sukker over `Iterator`-traiettens `next()`.

### 4.3 match (ekshaustivt)

```text
match value {
    0                => "nul"
    1 | 2 | 3        => "lille tal"
    n if n % 2 == 0  => "lige"
    n                => "ulige: {n}"
}

match point {
    Point(0, 0)       => "origo"
    Point(x, 0)       => "x-akse: {x}"
    Point(_, y) where y > 0 => "over"
    _                 => "andre"
}

match opt {
    some(v) => v
    none    => 0
}
```

Patterns (fuldt sammensætbare): literal, wildcard `_`, binding, tuple, struct `{name, age}`, enum `Variant(pats)`, range `1..=10`, array `[first, ...rest]`, slice `[a, b, ..]`, type-test `x is T`, or-pattern `a | b`, guard `where cond`. Compileren verificerer **ekshaustivitet** og død-gren.

Match er udtryk og skal producere samme type i alle arme (eller unit).

### 4.4 try / catch (panic-handlers)

```text
try {
    risky()
} catch e: PanicError {
    log(e.message)
} finally {
    cleanup()
}
```

Bruges kun til undtagelsestilfælde — ikke kontrolflow (se error_handling.md).

### 4.5 use (context management, RAII)

Python `with` ↔ Nova `use`:

```text
use f = File.open("data.txt") {
    contents = f.read_all()
}                                   # f.close() kaldes deterministisk her

use lock = mutex.acquire() { ... }  # lock frigives ved blok-slut
use (a, b) = acquire_two() { ... }  # destructurerende use
```

Krav: typen implementerer trait `Disposable { fn dispose(self) }`.

## 5. Funktioner

```text
fn add(a: i32, b: i32) -> i32 {
    a + b                            # sidste udtryk = returværdi
}

fn greet(name = "verden", times: i32 = 1) {     # default-værdier
    ("Hej {name}! " * times).trim()
}

greet(times = 3)                     # navngivne argumenter
```

### 5.1 Variadiske parametre

```text
fn sum(...nums: i32) -> i32 {        # positional variadic
    nums.iter().sum()                # nums er Array<i32>
}

fn config(**opts: dynamic) {         # keyword-collect → Map<String, dynamic>
    opts.timeout ?? 30
}

sum(1, 2, 3)
config(timeout = 5, retries = 2)     # named args samles i opts
```

Spread ved kald: `sum(...[1, 2, 3])`.

### 5.2 Lambdas og closures

```text
square = x => x * x
add    = (a, b) => a + b
body   => { let t = prep(); compute(t) }

factor = 3
scale  = x => x * factor             # closure fanger environment (by capture)
```

Capture-regel: default **by reference** med ARC; compiler kopierer automatisk hvis levetiden kræver det (escape). Eksplicit: `[x, &y] => ...` (capture by value / by ref).

Function types: `fn(i32, i32) -> i32`, nullable: `(fn(i32) -> bool)?`.

### 5.3 Overloads og generiske funktioner

```text
fn parse(s: String) -> i32 { ... }       # overloading tilladt på parametertyper
fn parse(s: String) -> f64 { ... }

fn max<T: Comparable>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn first<T>(xs: [T]) -> T? { xs.first() }   # [T] = Array<T> i generisk kontekst
```

Generics monomorphiseres (native backend); VM bruger type-erased + caches. Bounds via traits. Se type_system.md.

### 5.4 Rekursion og tail calls

Tail-call optimering garanteret for selv-rekursion markeret `@tailrec` (compiler-fejl hvis ikke tail-position).

```text
@tailrec
fn count(n: i32, acc: i32 = 0) -> i32 {
    if n == 0 { acc } else { count(n - 1, acc + n) }
}
```

### 5.5 Doc-kommentarer

```text
/// Returnerer arealet af en cirkel.
///
/// # Eksempel
/// area = circle_area(2.0)  // ≈ 12.566
fn circle_area(r: f64) -> f64 { PI * r ** 2 }
```

Doctests køres af `nova test --doc`.

## 6. Collections-operationer (stdlib-core)

Alle metoder der returnerer ny collection er lazy hvor muligt (iterator-baserede):

```text
xs = [5, 3, 8, 1]

# Transformér
xs.map(x => x * 2)                 # [10, 6, 16, 2]
xs.filter(x => x > 2)              # [5, 3, 8]
xs.rev()                           # [1, 8, 3, 5]
xs.sorted()                        # [1, 3, 5, 8]
xs.sorted(by = (a, b) => b <=> a)  # faldende
xs.zip([9, 8, 7])                  # [(5,9),(3,8),(8,7)]
xs.chunked(2)                      # [[5,3],[8,1]]
xs.windowed(2)                     # [[5,3],[3,8],[8,1]]
xs.flat_map(x => [x, x])           # [5,5,3,3,8,8,1,1]

# Reducér / forespørg
xs.sum()  xs.product()  xs.min()  xs.max()
xs.count(x => x > 2)  xs.any(x => x > 7)  xs.all(x => x > 0)
xs.fold(0, (acc, x) => acc + x)
xs.reduce((a, b) => a + b)
xs.find(x => x > 4)                # some(5) | none
xs.position(x => x == 8)           # Index? = some(2)
xs.group_by(x => x % 2)            # Map<Bool, Array>

# Indeks/slice (negativ indeks ^1 = sidste)
xs[0]  xs[^1]  xs[1..3]  xs[..2]  xs[2..]  xs[0.. by 2]
xs[1:3]                            # Python-stil sukker for 1..<3

# Mutation
xs.push(4)  xs.pop()  xs.insert(0, 9)  xs.remove_at(2)
xs.extend([7, 7])  xs.clear()  xs.sort()  xs.reverse()

# Medlemsskab/forekomst
3 in xs  xs.index_of(8)  xs.unique()  xs.contains_all([1, 3])
```

Map/Set:

```text
m = {"a": 1}
m["b"] = 2            m.get("c")           # none, ikke exception
m.get_or_insert("c", 0)
"b" in m              m.keys()  m.values()  m.items()
{k: v * 2 for (k, v) in m}                 # dict-comprehension
{x % 3 for x in 0..30}                     # set-comprehension
m.merge({"z": 9})     m | {"w": 0}         # union-operatorer
```

Comprehensions (generel form):

```text
[expr for x in iter if cond if cond2]
[k: v for (k, v) in pairs]
{elem for x in iter}
```

## 7. Strings

```text
s = "Verden"
name = "Nova"
msg = "Hej {name}, {1 + 1}"          # interpolation: Hej Nova, 2
pi_msg = "π ≈ {PI:.3}"               # format-spec (som Pythons :.3f)
raw = r"C:\sti\uden\escapes"
multi = """
    flere linjer
"""
bytes_s = b"ABC"                     # Bytes
ch = 'A'                             # char (Unicode scalar)

# Metoder (uddrag — fuld liste i standard_library.md §core.string)
s.lower()  s.upper()  s.trim()  s.strip(" ")
s.split(",")  s.rsplit(",", max = 1)  s.split_lines()
"-".join(["a", "b"])  s.replace("e", "E")  s.remove_prefix("He")
s.starts_with("V")  s.ends_with("n")  s.find("r")  s.count("e")
s.repeat(2)  s.pad_left(10)  s.center(20, '*')
s.chars()  s.bytes()  s.code_points()  s.graphemes()   # iteratorer
s.to::<i32>()  "3.14".to::<f64>()
s <=> "abc"                          # Unicode-korrekt kollation

# Slice som collections
s[0..3]  s[^5..]  s[::-1]            # baklænds via step -1
```

Strings er UTF-8; indeksering er **byte-indeks** og O(1) usikkert ved chars — derfor `s.chars()[i]` for positionel adgang (linter fanger `s[i]` på String).

## 8. Klasser, structs, enums, traits

### 8.1 struct (værditype, data)

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

Structs har værdi-semantik (copy eller move), ingen arv. Felter kan have defaults.

### 8.2 class (referencetype, arv)

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

    override fn sound() -> String { "Vuf" }
}

d = Dog("Rex", "labrador")
d.sound()
```

- Enkel arv (`:`), interfaces via traits.
- `virtual`/`override` er **påkrævede ord** — ingen tilfældig polymorfi.
- Felter: `pub`, `priv` (default), `protected`. Properties:

```text
class Account {
    priv balance_: f64 = 0

    balance: f64 {
        get { self.balance_ }
        set(v) {
            if v < 0 { throw Error("negativ saldo") }
            self.balance_ = v
        }
    }
}
```

- Static members: `static fn create()`. 
- Destructors: `fn deinit()` (kaldes deterministisk under ARC).
- Auto-generering: `@derive(Equals, Compare, Hashable, Printable, Serializable)`.

### 8.3 Dataklasser

```text
@dataclass
class Person {
    name: String
    age: i32 = 0
}
# giver gratis: init med named args, Equals, Hashable, Printable,
# copy-with, Serializable (se metaprogramming.md)
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

Enums kan have metoder, fælles felter og generics. Simple enums: `enum Color { Red | Green | Blue }`.

### 8.5 traits (interfaces + mixins)

```text
trait Drawable {
    fn draw(self, canvas: Canvas)       # påkrævet metode

    fn draw_twice(self, canvas: Canvas) {   # default-implementering
        self.draw(canvas)
        self.draw(canvas)
    }
}

impl Drawable for Circle {
    fn draw(self, canvas: Canvas) { canvas.circle(self.center, self.r) }
}
```

- Trait-objects (dynamisk dispatch): `dyn Drawable`.
- Generiske bounds: `fn render<T: Drawable>(items: Array<T>)`.
- Traits kan kræve associerede typer/konstanter og have blanket-impls (std-traits).
- Operator-overloading og `Iterable`, `Index`, `Comparable` osv. er almindelige traits.

### 8.6 Extension methods

```text
extend String {
    fn shout(self) -> String { self.upper() + "!" }
}

"hej".shout()
```

## 9. Moduler

```text
mod math_utils {
    pub fn helper() {}
    fn internal() {}                 # privat for modulet
}

import math_utils
import std.io.File
import std.json as json
from std.math import sqrt, PI

export                              # gør hele filens pub-symboler offentlige API
```

Detaljer i module_system.md.

## 10. Operator-overloading (traits-tabel)

| Udtryk | Trait der skal implenteres |
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
| `a < b` osv. | `Compare` (`fn cmp -> Ordering`) |
| `xs[i]` / `xs[i] = v` | `Index` / `IndexSet` |
| `xs[a..b]` | `Slice` |
| `for x in obj` | `Iterable` (`fn iterator`) |
| `obj(...)` | `Callable` (`fn call`) |
| `"x" * 3` | `Mul` med asymmetriske typer tilladt |
| `f"{obj}"` | `Printable` (`fn format(fmt) -> String`) |
| `truthy(obj)` / `if obj` | `Truthiness` (default: alt andet end `false`/`none`/`0`/`""` er sandt? NEJ — kun `bool` og `bool?` er betingelser; alt andet er compile-fejl) |

**Bevidst afvigelse fra Python:** betingelser kræver rigtig `bool`. Linter kan slappe af lokalt (`#nova allow truthiness`). Dette fjerner en hel klasse af `=` vs `==`/tomhed-fejl.

## 11. Iterators og generators

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

`yield` omskrives af compileren til en state machine (som async). Generatorer er bare funktioner der returnerer `Iterator<T>`. Iterator-chain er zero-cost efter inlining (monomorphiseret).

Iterator-adaptere (fuld liste i stdlib): `map filter take skip take_while skip_while enumerate zip chain interleave flat_map scan fuse peekable chunked windowed step_by rev sorted unique group_by product permutations combinations`.

Terminal-operationer: `collect to_array to_set to_map sum min max count any all find fold reduce for_each join partition_by`.

## 12. Fejlhåndtering (kort — fuldt i error_handling.md)

```text
fn read_config(path: String) -> Result<Config, ConfigError> {
    text = File.read(path)?                  # propagerer
    parsed = json.parse(text)?
    Config.from_json(parsed)
}

port = env.get("PORT").and_then(v => v.parse::<i32>().ok()) ?? 8080

assert(xs.len() > 0, "listen må ikke være tom")   # panic i debug, no-op i release
require(input != null, "input er påkrævet")        # altid aktiv guard
```

## 13. Async og parallel (kort — fuldt i concurrency.md)

```text
async fn fetch(url: String) -> Bytes {
    resp = await http.get(url)
    resp.body
}

async fn fetch_all(urls: Array<String>) {
    results = await gather(urls.map(fetch))     # konkurrentielt
    ...
}

parallel {
    a = calculate_a()
    b = calculate_b()
}                                                # a og b kører samtidigt
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
q = t.construct({name: "Anna"})      # dynamisk konstruktion

if obj is Drawable { obj.draw(c) }   # typetest + cast
d = obj as dyn Drawable              # trait-object cast (Result ved fejl)
```

Runtime-metadata inkluderes i `--runtime full`; i minimal strippes den (reflection-kald = compile-fejl).

## 15. Attributes (decorators/metadata)

```text
@test
fn addition_works() {
    expect(add(1, 2) == 3)
}

@deprecated("brug new_api()")
fn old_api() {}

@benchmark(warmup = 100)
fn matrix_bench() {}

@generate_serialization              # makro (metaprogramming.md)
struct Order { id: Uuid, total: f64 }

@inline @cold @simd @checked @pure @noinline
@gpu(block = (256, 1, 1)) @thread_local @volatile
```

Attributes med argumenter kan være vilkårlige compile-time-udtryk. Brugerdefinerede attribute-makroer transformeres i AST (se metaprogramming.md).

## 16. Compile-time evaluering

```text
const TABLE = [x ** 2 for x in 0..256]      # beregnes ved kompilering

@compile
fn gen_primes(limit: i32) -> Array<i32> { ... }

const PRIMES = gen_primes(100)
```

Alt hvad der er `@pure`-kompatibelt kan køres compile-time: loops, match, generics — ikke IO/tråde/dynamic.

## 17. Unsafe og raw memory (kort — fuldt i memory_model.md)

```text
unsafe {
    buf = malloc(n)
    defer { free(buf) }                  # defer: kaldes ved scope-exit, LIFO
    ptr = &buf[0]
    *ptr = 42
}
```

`defer` findes også i safe kode (scope-bundet cleanup uden trait-krav).

## 18. Scripting og shebang

```text
#!/usr/bin/env nova
print("script!")
args = CLI.args()                        # kommandolinjeargumenter
exit(CLI.exit_code)
```

`nova run script.nova` starter VM'en direkte (ingen link-fase). REPL: `nova repl`.

## 19. Reserverede keywords (komplet)

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
_ (wildcard er ikke keyword men symbol)
```

`null` findes kun i unsafe/FFI-kontekst (raw pointers).

## 20. Pipelines — BESLUTTET

```text
# Kompakt
contents |> split_lines() |> filter(l => l.len() > 0) |> sorted() |> join("\n") |> print()
```

```text
# Natural — læses som en sætning:
take the file contents
    then split it by lines
    then keep the ones that are not empty
    then sort them
    then say the result
done
```

Desugares til almindelige metode-kæder (zero-cost efter inlining). `|>` sender venstre værdi som første argument til højre kald. Natural-fraserne (`then split it by`, `then keep the ones that`, `then turn every X into`) er faste skabeloner i natural_syntax.md-ordbogen.

## 21. Signals (reaktiv tilstand) — BESLUTTET

```text
score = signal(0)
rank  = computed(() => "Niveau {score.value / 100}")
effect { print("{score.value} → {rank.value}") }     # genkører ved ændring
score.value += 60                                     # → automatisk opdatering
```

Natural:

```text
the score is a signal starting at 0
when the score changes
    say "{score} → {rank}"
done
add 50 to the score
```

Semantik:

- Pull-baseret, glitch-fri topologisk invalidering (SolidJS/SwiftUI-model): afledte værdier om-beregnes kun når læst, og aldrig i mellem-tilstande.
- `computed` memoizer; afhængighedssporing er automatisk (ingen dependency-lister).
- `effect` kører efter commit; exceptions i effects paniker normalt.
- GUI-integration: nova-gui binder direkte — `the label text binds to the rank`.
- Async-kilder: `stream.into_signal()` driver en signal fra events/netværk.

## 22. Actors — BESLUTTET

Fuld semantik: concurrency.md §7. Kortform:

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

Én besked ad gangen pr. actor → ingen locks; felter kan kun berøres fra egne handlers.

## 23. Contracts — BESLUTTET

```text
fn withdraw(amount: f64) {
    requires(amount > 0)
    requires(amount <= self.balance)
    ensures(self.balance >= 0)

    self.balance -= amount
}
```

Natural: `requires amount is greater than 0` / `ensures my balance is at least 0`.

Regler:

- `requires` evalueres ved indgang, `ensures` ved udgang (har adgang til returværdien via `result`).
- Tjekkes i debug/tests; strippes i release. Profil: `contracts = "debug" | "always" | "never"`.
- Kontraktsudtryk skal være `@pure`.
- Samme kontrakter driver: fuzzing-input-generering (testkit), docs/hover-visning og refinement-verificeringen (type_system §11).
