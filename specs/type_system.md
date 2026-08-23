# Nova Type System

## 1. Design goals

1. Static by default — everything is typed at compile time.
2. Inference so comprehensive that you rarely write types in local code.
3. `dynamic` as a first-class escape hatch, without weakening the rest.
4. Generics with trait bounds, monomorphized native, type-erased in the VM.

## 2. Inference

**Local Hindley-Milner-inspired + annotation-guided:**

```text
x = 10                    # i32
name = "Carl"             # String
nums = [1, 2, 3]          # Array<i32>
mixed = [1, "a"]          # ERROR: homogeneous collections; use [i32 | String] or dynamic
f = x => x * 2            # parameters: context-driven (call site or annotation)
```

Rules:

- Int literals default to `i32`, float literals to `f64` (literal-based polymorphism: `let a: u8 = 5` is fine).
- Lambda parameters are inferred from context (assignment-target type, call-site generic instantiation, iterator element type). Otherwise an annotation is required: `(x: f64) => ...`.
- Function and class APIs: **explicit types required on pub symbols** (hard rule). Private helpers may be fully inferred.
- Recursive functions require a return-type annotation when non-trivial (circular inference is broken with a helpful error).
- Inference runs **before** trait resolution; error messages point at the failing constraints.

## 3. Type hierarchy

```text
Any (the top, dynamic only)
├── primitive (ints, floats, bool, char)
├── String
├── struct/enum instances (value types)
├── class instances (reference types)
├── Array<T> Map<K,V> Set<T> Tuple ...
├── fn-signaturer
├── dynamic
└── never (the bottom — throw/infinite loop)

Union: A | B   (anonymous sum type)
Optional: T? ≡ T | None
```

Subtyping:

- Class inheritance: `Dog <: Animal`.
- Traits: any type implementing `T` is valid where `dyn T` is expected (auto-boxing).
- Union: `A <: A | B`. Narrowing via `is`/match.
- No implicit numeric conversion (except literal adaptation) — `as` is explicit.

## 4. Unions

```text
fn parse_id(s: String) -> i32 | String {     # either an id or an error message
    if s.is_digits() { s.parse::<i32>() } else { "not a number" }
}

v = parse_id(input)
match v {
    n is i32    => print(n * 2)
    msg         => print(msg)                # the rest type
}
```

Unions collapse/flatten (`i32 | i32` = `i32`; `(A|B)|C` = `A|B|C`). `T?` is syntactic sugar for `T | None`. Exhaustive match on unions is required.

## 5. Generics

```text
fn max<T: Comparable>(a: T, b: T) -> T

struct Box<T> { value: T }

impl<T: Printable> Printable for Box<T> {
    fn format(self) -> String { "Box({self.value})" }
}

fn pair_min<T: Comparable>(xs: Array<T>) -> T?   # bounds are traits
```

- Bounds: `T: Trait1 + Trait2`, default bounds via `where` clauses.
- Associated types i traits: `trait Container { type Item; fn get(self, i: i64) -> Item }`.
- Default-typeparametre: `struct Result<T, E = Error>`.
- Variance: structs/enums invariant; references covariant (read-only refs covariant, mutable invariant).
- Native backend: monomorphization + deduplication of identical instantiations.
- VM: type-erased with dynamic-dispatch caches (same semantics).
- Recursion limits enforced (deep monomorphization chains give a clear error).

## 6. Traits and coherence

- Coherence rule: one impl per `(trait, type)` per program (except local impls in non-exported modules — orphans allowed locally).
- Blanket impls in stdlib: `impl<T: Comparable> Comparable for Array<T>` etc.
- Trait objects: `dyn Trait` = fat pointer (data + vtable). Auto-boxing on coercion.
- Dynamic vs static dispatch: methods on classes are virtual if `virtual`; trait calls on generic parameters are static; on `dyn Trait` dynamic.

## 7. dynamic — complete semantics

```text
d: dynamic = {"name": "Nova", "version": 1}
d.name                       # → dynamic ("Nova")
d.version as i32             # explicit down-conversion
d.missing ?? "default"       # coalescing works
d.liste?[0]                  # optional-chaining gennem dynamic
```

Semantics:

- `dynamic` = tagged value: `int | float | string | bool | none | array<dynamic> | map<String,dynamic> | callable | object-ref`.
- Member access/index/call checked at runtime. Error = panic `DynamicError` (catchable with try/catch) — or propagate as Result with a `?.`-style API.
- Interop with static code: all types coerce automatically into dynamic; out requires `as T` (checked) or `.to::<T>()` (Result).
- Method calls on dynamic: inline caches in the VM, megamorphic fallback natively.
- The compiler tracks "dynamic contamination": if an expression touches a dynamic value the result becomes dynamic (no half-dynamic types).
- JSON/JSON-like data is the primary use case; `Map<String, dynamic>` is the JSON model.

## 8. Numeric rules

| Operation | Regel |
|---|---|
| int op int | same width; mixed widths = compile error (use `as`) |
| float op int | error — int must be converted explicitly (`x.to_f64()`) |
| `/` | real division; int/int = error — use `/` on floats or `//` floor-div on ints |
| `%` | sign follows the dividend (Python semantics); `mod()` available for C semantics |
| `**` | int**int → error on negative exponent; float otherwise |
| overflow | debug: panic; release: wrapping; `@checked`: always panic; checked-arithmetic API: `add_checked -> Result` |

Rationale: silent promotion hides bugs (JS/PHP heritage); Python-style willingness to handle big numbers lives in `BigInt`.

## 9. Type aliases and newtypes

```text
type Matrix = Array<Array<f64>>
type Callback = fn(Event) -> ()

@newtype Meters(f64)        # newtypes: zero-cost wrapper with its own impl scope
impl Compare for Meters { ... }
```

## 10. Compatibility matrix (coercions)

| From \ To | superclass | dyn Trait | union member | dynamic | raw ptr |
|---|---|---|---|---|---|
| subklasse | implicit | auto-box | hvis medlem | implicit | unsafe |
| konkret type | — | auto-box | hvis medlem | implicit | unsafe |
| dynamic | runtime-check | runtime-check | runtime-check | — | unsafe |

Runtime checks use type metadata (full runtime) or vtable ids (minimal).

## 11. Refinement types (conditional types) — DECIDED

```text
type Age = i32 where self >= 0 && self <= 130        # compact form
an age is a whole number from 0 to 130               # natural form

a Positive is a number greater than 0
type NonEmpty<T> = Array<T> where self.len() > 0
```

Rules:

- A refinement type is a subtype of its base type with a predicate.
- **Boundary checks:** values convert into the type with a check (`x as Age` → runtime predicate check; constant arguments breaking the predicate = compile error).
- Inside a function taking a refinement-typed parameter the predicate is assumed — zero cost when flow analysis can prove it, otherwise an automatic boundary check at call.
- Predicates must be `@pure` and may only use parameters, constants and `@pure` stdlib calls.
- The compiler's SMT-light module proves where possible that checks are redundant and removes them.

Refinement types = data invariants; contracts (language_reference §23) = control-flow requirements. They share one verification engine.

## 12. Units and dimensions — DECIDED

```text
let d = 100.m                 # Unit<Length>
let t = 9.58.s                # Unit<Time>
let v = d / t                 # Unit<Length/Time> — inferred
say "{v.in::<km/h>()}"        # conversion

d + t                         # COMPILE ERROR: dimensions do not match

# Natural
the distance is 100 meters
the speed is the distance divided by the time
say "{the speed in kilometers per hour}"
```

- Implementation: generic `Unit<Base, Dim-exponents>`; monomorphizes to raw `f64` — **zero runtime cost**.
- `std.units` (see standard_library §21): SI base units, derived units, prefixes, conversions, physics constants with correct dimensions.
- Currency = `Decimal` + unit tag; exchange rates are always explicit conversions.
- Units inside `dynamic` degrade to runtime dimension checks.
- Linter warning on implicit unit loss (`.value` on a Unit without `in::<>()`).
