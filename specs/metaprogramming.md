# Nova Metaprogramming & Reflection

## 1. Compile-time evaluation

```text
const SIZE = 1024
const LOOKUP = [x * x for x in 0..SIZE]        # computed at compile time

@compile                                        # function may run at compile time
fn sieve(limit: i32) -> Array<i32> { ... }

const PRIMES = sieve(1000)
```

- Anything that is `@pure` (no IO, threads, dynamic, global mutation) can be evaluated at compile time.
- `@compile` on a fn = contract: calls with constant arguments are folded; otherwise runtime.
- Compile-time debugging: `comptime print(...)` writes during compilation; errors in comptime code get normal spans.

## 2. Derive-attributter

```text
@derive(Equals, Compare, Hashable, Printable, Serializable, Clone)
struct Point { x: f64, y: f64 }
```

Generates impls in the IR phase (visible to LSP as expanded code).

## 3. Makroer

### 3.1 Function-like macros (AST → AST)

```text
macro sql(query: StringLiteral) -> Expr {
    # runs at compile time, receives token stream/AST
    # returns an expression (here: validated query builder)
    parse_and_validate_sql(query)   # pseudo — the macro body is itself Nova code
}

users = sql!("SELECT id, name FROM users WHERE age > {min_age}").all(db)
```

- Hygienic (identifiers in macro output do not collide with the caller scope).
- Can take types as input: `macro RowOf(T: Type) -> Type`.
- Debuggable: `nova expand file.nova` shows the expansion.

### 3.2 Attribute macros / code generation

```text
@generate_serialization
struct Order {
    id: Uuid
    total: f64
}
# generates: to_json(), from_json(), serialize(stream), deserialize(stream),
# schema() (reflection format), roundtrip test

@generate_builder
class HttpRequest { url: String; method: String = "GET"; ... }
# provides HttpRequest.builder().url(...).method(...).build() -> Result
```

Macro API: macros are programmed in Nova against the compiler's AST library (`nova.ast`) and run in a sandboxed comptime interpreter. No native plugin burden.

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

Dynamic behavior:

```text
p = Person(name = "Carl", age = 30)
p["name"]                       # dynamic value via reflection
p["age"] = 31                   # checked set (Result on type error)

inst = t.construct({name: "Anna"})          # dynamic construction
copy = p.clone_shallow()

# Generic serialization built on reflection:
fn to_json_any(v: Any) -> String { ... }
```

- Metadata exists in `--runtime full`; `--runtime minimal` strips it (reflection calls = link error).
- Reflection is statically verified where possible: `typeof(X).field("name")` fails at compile time if the field does not exist.

## 5. Trait introspection

```text
if obj implements Drawable { obj.draw(canvas) }     # runtime trait check
dyn_obj = obj as dyn Drawable?                      # optional cast
```

## 6. Compiler plugin boundaries (what is NOT possible)

- No arbitrary syntax modifications (the grammar is fixed — deliberately).
- Macros cannot bypass type safety or introduce unsafe without an `unsafe` block.
- Build scripts run sandboxed (fs access limited to the package directory).
