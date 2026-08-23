# Nova Typesystem

## 1. Designmål

1. Statisk som default — alt er typet ved kompilering.
2. Inference så omfattende at man sjældent skriver typer i lokal kode.
3. `dynamic` som førsteklasses escape-luge, uden at svække resten.
4. Generics med traits-bounds, monomorphiseret native, type-erased i VM.

## 2. Inferencen

**Lokal Hindley-Milner-inspireret + annotation-guidet:**

```text
x = 10                    # i32
name = "Carl"             # String
nums = [1, 2, 3]          # Array<i32>
mixed = [1, "a"]          # FEJL: homogene collections; brug [i32 | String] eller dynamic
f = x => x * 2            # parametre: kontekststyret (kaldesitet eller annotation)
```

Regler:

- Int-literals default til `i32`, float-literals til `f64` (litteral-baseret polymorfi: `let a: u8 = 5` ok).
- Lambda-parametre infereres fra kontekst (assignment-target type, kald-site generic instantiation, iterator-elementtype). Ellers kræves annotation: `(x: f64) => ...`.
- Funktions- og klasse-API'er: **eksplicitte typer påkrævet på pub-symboler** (hård regel). Private helpers må være fuldt infererede.
- Rekursive funktioner kræver returtype-annotation hvis ikke-trivial (cirkulær inference afbrydes med hjælpsom fejl).
- Inference kører **før** trait resolution; fejlmeddelelser peger på constraints der fejlede.

## 3. Typehierarki

```text
Any (toppen, kun dynamisk)
├── primitive (ints, floats, bool, char)
├── String
├── struct/enum-instanser (værdityper)
├── class-instanser (referencetyper)
├── Array<T> Map<K,V> Set<T> Tuple ...
├── fn-signaturer
├── dynamic
└── never (bunden — throw/udødelig loop)

Union: A | B   (anonym sumtype)
Optional: T? ≡ T | None
```

Subtyping:

- Klasses-arv: `Dog <: Animal`.
- Traits: enhver type der implementerer `T` er gyldig hvor `dyn T` forventes (auto-boxing).
- Union: `A <: A | B`. Narrowing via `is`/match.
- Ingen implicit numerisk konvertering (undtagen literal-adaptation) — `as` er eksplicit.

## 4. Unions

```text
fn parse_id(s: String) -> i32 | String {     # enten id eller fejlbesked
    if s.is_digits() { s.parse::<i32>() } else { "ikke et tal" }
}

v = parse_id(input)
match v {
    n is i32    => print(n * 2)
    msg         => print(msg)                # rest-typen
}
```

Unions kollapser/fladtes (`i32 | i32` = `i32`; `(A|B)|C` = `A|B|C`). `T?` er syntakt sukker for `T | None`. Ekshaustivt match på unions er påkrævet.

## 5. Generics

```text
fn max<T: Comparable>(a: T, b: T) -> T

struct Box<T> { value: T }

impl<T: Printable> Printable for Box<T> {
    fn format(self) -> String { "Box({self.value})" }
}

fn pair_min<T: Comparable>(xs: Array<T>) -> T?   # bounds er traits
```

- Bounds: `T: Trait1 + Trait2`, default bounds via `where`-klausuler.
- Associated types i traits: `trait Container { type Item; fn get(self, i: i64) -> Item }`.
- Default-typeparametre: `struct Result<T, E = Error>`.
- Variance: structs/enums invariant; referencer covariante (read-only refs covariante, mutable invariant).
- Native backend: monomorphisering + deduplikering af identiske instantiationer.
- VM: type-erased med dynamic-dispatch caches (samme semantik).
- Recursion limits håndhævet (dybe monomorphiseringskæder giver klar fejl).

## 6. Traits og coherence

- Coherence-regel: én impl per `(trait, type)` pr. program (undtagen lokale impls i moduler der ikke eksporteres — orphans tillades lokalt).
- Blanket impls i stdlib: `impl<T: Comparable> Comparable for Array<T>` osv.
- Trait-objects: `dyn Trait` = fat pointer (data + vtable). Auto-boxing ved coercion.
- Dynamisk dispatch vs statisk: metoder på klasser er virtuelle hvis `virtual`; trait-kald på generiske parametre er statiske; på `dyn Trait` dynamiske.

## 7. dynamic — komplet semantik

```text
d: dynamic = {"navn": "Nova", "version": 1}
d.navn                       # → dynamic ("Nova")
d.version as i32             # eksplicit nedkonvertering
d.mangler ?? "default"       # coalescing virker
d.liste?[0]                  # optional-chaining gennem dynamic
```

Semantik:

- `dynamic` = tagged value: `int | float | string | bool | none | array<dynamic> | map<String,dynamic> | callable | object-ref`.
- Medlemsadgang/indeks/kald checkes runtime. Fejl = panik `DynamicError` (kan fanges med try/catch) — eller propagér som Result med `?.`-stil API.
- Interop med statisk kode: alle typer coerces automatisk ind i dynamic; ud kræves `as T` (checked) eller `.to::<T>()` (Result).
- Metodekald på dynamic: inline caches i VM, megamorphic fallback i native.
- Compileren sporer "dynamic-kontaminering": hvis et udtryk berører en dynamic-værdi bliver resultatet dynamic (ingen halvdynamiske typer).
- JSON/JSON-lignende data er den primære use case; `Map<String, dynamic>` er JSON-modellen.

## 8. Numerik-regler

| Operation | Regel |
|---|---|
| int op int | samme bredde; mixed widths = compile-fejl (brug `as`) |
| float op int | fejl — int skal konverteres eksplicit (`x.to_f64()`) |
| `/` | ægte division; int/int → fejl hvis begge ints? Nej: int/int = fejl, brug `/` på floats eller `//` floor-div på ints |
| `%` | sign følger dividend (Python-semantik); `mod()` tilgængelig for C-semantik |
| `**` | int**int → fejl ved negativ eksponent; float ellers |
| overflow | debug: panik; release: wrapping; `@checked`: altid panik; checked-aritmetik API: `add_checked -> Result` |

Begrundelse: stille promotion skjuler bugs (JS/PHP-arv); Python-agtig vilje til store tal lever i `BigInt`.

## 9. Type-aliaser og nye typer

```text
type Matrix = Array<Array<f64>>
type Callback = fn(Event) -> ()

@newtype Meters(f64)        # nytyper: zero-cost wrapper med egen impl-scope
impl Compare for Meters { ... }
```

## 10. Kompatibilitetsmatrice (coercions)

| Fra \ Til | superklasse | dyn Trait | union-medlem | dynamic | raw ptr |
|---|---|---|---|---|---|
| subklasse | implicit | auto-box | hvis medlem | implicit | unsafe |
| konkret type | — | auto-box | hvis medlem | implicit | unsafe |
| dynamic | runtime-check | runtime-check | runtime-check | — | unsafe |

Runtime-checks bruger type-metadata (full-runtime) eller vtable-id (minimal).

## 11. Refinement types (betingede typer) — BESLUTTET

```text
type Age = i32 where self >= 0 && self <= 130        # kompakt form
an age is a whole number from 0 to 130               # natural form

a Positive is a number greater than 0
type NonEmpty<T> = Array<T> where self.len() > 0
```

Regler:

- En refinement-type er en subtype af sin basistype med et predikat.
- **Grænse-tjek:** værdier konverteres ind i typen med check (`x as Age` → runtime-predikat-tjek; konstante argumenter der bryder predikatet = compile-fejl).
- Inden i en funktion med parameter af refinement-typen er predikatet antaget — zero-cost når flow-analysen kan bevise det, ellers automatisk grænsecheck ved kald.
- Predikater skal være `@pure` og må kun bruge parametre, konstanter og `@pure` stdlib-kald.
- Compilerens SMT-light-modul beviser hvor muligt at checks er overflødige og fjerner dem.

Refinement-typer = data-invarianter; contracts (language_reference §23) = kontrolflow-krav. De to deler verificeringsmotor.

## 12. Enheder og dimensioner (units) — BESLUTTET

```text
let d = 100.m                 # Unit<Length>
let t = 9.58.s                # Unit<Time>
let v = d / t                 # Unit<Length/Time> — infereret
say "{v.in::<km/h>()}"        # konvertering

d + t                         # COMPILE-FEJL: dimensions matcher ikke

# Natural
the distance is 100 meters
the speed is the distance divided by the time
say "{the speed in kilometers per hour}"
```

- Implementering: generisk `Unit<Base, Dim-exponents>`; monomorphiserer til rå `f64` — **nul runtime-omkostning**.
- `std.units` (se standard_library §21): SI-basisenheder, afledte enheder, præfikser, konverteringer, fysikkonstanter med korrekte dimensioner.
- Valuta = `Decimal` + enhedstag; valutakurser er altid eksplicitte konverteringer.
- Enheder i `dynamic` degraderer til runtime-dimensionstjek.
- Linter-advarsel ved implicit tab af enhed (`.value` på Unit uden `in::<>()`).
