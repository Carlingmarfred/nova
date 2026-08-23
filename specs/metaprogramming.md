# Nova Metaprogramming & Reflection

## 1. Compile-time evaluering

```text
const SIZE = 1024
const LOOKUP = [x * x for x in 0..SIZE]        # beregnet ved kompilering

@compile                                        # funktion kan køre compile-time
fn sieve(limit: i32) -> Array<i32> { ... }

const PRIMES = sieve(1000)
```

- Alt der er `@pure` (ingen IO, tråde, dynamic, global mutation) kan evalueres compile-time.
- `@compile` på en fn = kontrakt: kald med konstante argumenter foldes; ellers runtime.
- Compile-time-debugging: `comptime print(...)` skriver under kompilering; fejl i comptime-kode får almindelige spans.

## 2. Derive-attributter

```text
@derive(Equals, Compare, Hashable, Printable, Serializable, Clone)
struct Point { x: f64, y: f64 }
```

Genererer impls i IR-fasen (synlige for LSP som expandet kode).

## 3. Makroer

### 3.1 Funktion-agtige makroer (AST → AST)

```text
macro sql(query: StringLiteral) -> Expr {
    # kører compile-time, modtager token-stream/AST
    # returnerer udtryk (her: verificeret query-builder)
    parse_and_validate_sql(query)   # pseudo — makrokroppen er selv Nova-kode
}

users = sql!("SELECT id, name FROM users WHERE age > {min_age}").all(db)
```

- Hygiejniske (identifiers i makro-output kolliderer ikke med kalder-scope).
- Kan modtage typer som input: `macro RowOf(T: Type) -> Type`.
- Debugbar: `nova expand fil.nova` viser ekspansion.

### 3.2 Attribute-makroer / code-generering

```text
@generate_serialization
struct Order {
    id: Uuid
    total: f64
}
# genererer: to_json(), from_json(), serialize(stream), deserialize(stream),
# schema() (reflection-format), roundtrip-test

@generate_builder
class HttpRequest { url: String; method: String = "GET"; ... }
# giver HttpRequest.builder().url(...).method(...).build() -> Result
```

Makro-API: makroer er programmeret i Nova mod compilerens AST-bibliotek (`nova.ast`) og køres i sandboxed comptime-tolk. Ingen C++-plugin-byrde.

## 4. Reflection (runtime)

```text
t = typeof(Person)
t.name                    # "Person"
t.kind                    # .struct | .class | .enum | .trait ...
for f in t.fields {
    "{f.name}: {f.type} (default: {f.default})"
}
t.methods().map(m => m.signature)
t.attributes              # alle @-attributter med args
```

Dynamisk adfærd:

```text
p = Person(name = "Carl", age = 30)
p["name"]                       # dynamic-værdi via reflection
p["age"] = 31                   # checked set (Result ved typefejl)

inst = t.construct({name: "Anna"})          # dynamisk konstruktion
copy = p.clone_shallow()

# Generisk serialisering bygget på reflection:
fn to_json_any(v: Any) -> String { ... }
```

- Metadata findes i `--runtime full`; `--runtime minimal` stripper den (reflection-kald = link-fejl).
- Reflection er statisk verificeret hvor muligt: `typeof(X).field("navn")` fejler compile-time hvis feltet ikke eksisterer.

## 5. Traits-introspektion

```text
if obj implements Drawable { obj.draw(canvas) }     # runtime trait-check
dyn_obj = obj as dyn Drawable?                      # optional cast
```

## 6. Compiler-plugin-grænser (hvad der IKKE kan)

- Ingen vilkårlige syntax-modifikationer (grammatikken er fast — bevidst).
- Makroer kan ikke omgå typesikkerhed eller introducere unsafe uden `unsafe`-blok.
- Build-scripts kører i sandbox (fs-adgang begrænset til pakke-mappen).
