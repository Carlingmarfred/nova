# Nova Memory Model

## 0. Bootstrap-udsnit (v0.14+, item C13)

Native ARC/escape-analysis er et IR-pass (E05, gate G3). Bootstrap'en fastlægger
NU allerede den semantik, ARC'en senere skal præservere — og giver én eksplicit
kopierings-frase:

### 0.1 Værdi- vs. reference-semantik (gælder ALTID, også i native)

| Værdi | Semantik | Assignment/kald |
|---|---|---|
| tal, tekst, bool | **værdi** | kopieres (uafhængig) |
| `nothing` | værdi (én instans) | — |
| liste `[...]` | **reference** | deles (alias) |
| thing-instans | **reference** | deles (alias) |
| databog (json-objekt) | **reference** | deles (alias) |

Garanti: `ys is xs` hvor xs er liste/thing betyder ALDRIG kopi — ændringer via
`ys` ses gennem `xs`. Vil man have uafhængighed, skal man bede om det eksplicit.

### 0.2 `a copy of X`

```text
ks is a copy of xs          # dyb kopi — hele træet (nøstedede lister/things)
t2 is a copy of t           # ny ThingInstance, samme felt-værdier (dybt kopieret)
```

Regler:
1. `a copy of X` er ét udtryk (primær-frase); X parses grådigt som resten af
   aritmikken (samme konvention som `the contents of ...`).
2. Kopien er DYB: indlejrede lister/things/databøger kopieres rekursivt.
3. Kombineres frit med `?`: `a copy of maybe?` → nothing hvis maybe er nothing
   (hele-udtryksgift gælder).
4. Moduler/funktioner kan ikke kopieres — venlig fejl ("et modul er ikke en værdi").
5. Begge skins, identisk AST (`CopyOf`-node; golden 20 + kryds-skin-par 8).

Ikke i bootstrap-udsnittet (kommer i native-pipelinen): refcounts der kan aflæses,
`owned`/move/borrow-checking, `unsafe`, cycle collector, `deinit`.

## 1. Tre niveauer

```text
Default:      ARC (automatic reference counting) + escape analysis
Opt-in:       owned / move / borrow-annoteringer (zero-cost kontrol)
Ekspert:      unsafe + raw pointers
Valgfrit:     GC-cycle collector (--runtime full)
```

## 2. Værdityper vs referencetyper

| | struct, enum, tuple, [T;N], primitives | class-instans, closures, dyn Trait, Box |
|---|---|---|
| Semantik | værdi (copy hvis Copy, ellers move) | reference (ARC-talt) |
| Layout | inline, stack/embedded | heap-allokeret, talt pointer |
| Kopiér | `.clone()` eller Copy | deling (increment refcount) |

Copy-typer: alle primitives, pointers, arrays/tuples/structs af Copy-felter. Copy er opt-out (`@no_copy`). Move gør kilden ugyldig (compile-fejl ved brug efter move).

## 3. ARC-detaljer

- Hvert heap-objekt har refcount (+ weak count).
- Retain/release indsættes som IR-pass; escape analysis fjerner par i samme funktion.
- Release kan inline' destruktor; deallokering deterministisk ved sidste release.
- Tråd-sikkerhed: refcounts er atomare som default; single-threaded objekter (`@local`) bruger non-atomare counts (scheduleren garanterer isolering).

### Cyklusser

- `--runtime full`: inkrementel cycle collector (Bacon-Rajan-stil) kører i baggrunden; latensbundet.
- `--runtime minimal/core`: cykler lækker bevidst; `weak` bryder dem:

```text
class Node {
    parent: Node?              # stærk
    children: Array<Node>
}
n.parent = some(n)             # cyklus → collector rydder i full; ellers leak
weak_parent: weak Node?        # svag reference, upgrade: .upgrade() -> Node?
```

Linter advarer om oplagte cyklusser (statisk felt-analyse).

## 4. owned / move / borrow (avanceret)

```text
owned buf = Buffer(1 << 20)    # eksplicit ejerskab, ingen refcount
send(buf)                      # MOVE: buf er nu ugyldig

fn process(b: Buffer) {...}    # tager ejerskab (move ind)
fn peek(&buf: Buffer) {}       # låner (borrow), read-only, & = delt lån
fn edit(mut &buf: Buffer) {}   # mutbart lån
```

Ownership-reglerne (Rust-inspireret men **opt-in og mildere**):

- En værdi har én ejer; assignment/kald = move (for non-Copy).
- Mange delte lån ELLER ét mutbart lån ad gangen — håndhævet kun for typer markeret `owned`/`mut &`.
- Almindelige ARC-klasser kan stadig deles frit (ingen borrow checker på dem).

Dette giver Rust-agtig kontrol hvor man vil have det, uden at gøre hele sproget verbost.

## 5. unsafe

```text
unsafe {
    p = malloc(1024)
    defer free(p)
    q = p.offset(8)
    *q = 42u8
    raw = addr_of(x)           # *T
    arr = slice_from_raw(raw, 16)
}
```

- Raw pointers: `*T`, `*mut T`, `null`.
- Unsafe blokke er de eneste steder med deref/addr-of/malloc/FFI.
- `unsafe` smitter ikke (ingen unsafe-supertype); funktioner der kræver unsafe markeres `@unsafe fn`.
- Debug-builds: pointer-sanitizer-agtige checks (poisoning) når muligt.

## 6. Stack og heap

- `[T; N]`, structs, tuples: stack/embedded når muligt (escape analysis promoverer til heap ved flugt).
- Store objekter (> trøskel) placeres heap direkte.
- SOO (small object optimization): Optional<T> uden ekstra tag for reference-/niche-typer.

## 7. Levetider

Levetider infereres (non-lexical). Eksplicit lifetime-syntax findes IKKE i v1 — borrow-checkeren bruger regions-inference; hvor det er umuligt kræves `.clone()` eller ARC-delning i stedet. Dette er en bevidst forenkling ift. Rust: sproget vælger ARC som udvej i stedet for lifetime-annoteringer.

## 8. Finalization-rækkefølge

1. `defer` (scope-exit, LIFO)
2. `use`-dispose (ved blok-slut)
3. ARC-release → `deinit`
4. Cycle collector (kun full-runtime, ikke-deterministisk)

## 9. Embedded/minimal-profil

- Ingen cycle collector, ingen reflection-metadata, statiske allokatorer tilladt (`--alloc static`).
- `Box` erstattet af pool-allocator i `--alloc arena`.
- Interrupt-handlers: `@interrupt fn` (no-allokations-verificeret af compileren).
