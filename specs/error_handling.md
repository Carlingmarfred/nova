# Nova Error Handling

## 1. Filosofi

- **Forventede fejl** (fil findes ikke, netværk nede, bad input) = værdier: `Result<T,E>`, `Optional<T>`.
- **Uoprettelige fejl** (brudt invariant, OOM, indeks-fejl) = panic (unwind eller abort).
- Exceptions (`try/catch`) findes men er panic-handling — aldrig kontrolflow. Linter flagger catch i normal flow-logik.

## 2. Optional<T> ≡ T?

```text
find_user(id) -> User?

u = find_user(42)
if u == none { return }
print(u.name)                 # flow-typing: u er User her

u?.email?.primary             # chaining → String?
name = u?.name ?? "anonym"    # coalescing
forced = u!                   # unwrap-or-panic (kun med god grund; lint)
```

Flow-sensitive typing: efter `if x == none { return }` er `x` kendt non-none. Gælder også efter `?`, assert, match.

### 2.1 Bootstrap-udsnit (v0.11+, item C03) — IMPLEMENTERET i bootstrap v0.12

Bootstrap'en implementerer én præcis delregel af `?` — **hele-udtryksgift**:

> Hvis NOGEN del af et udtryk er `nothing`, og udtrykket indeholder en `?`,
> bliver hele udtrykket `nothing`. Uden `?` fejler det som altid med en
> venlig fejlmeddelelse.

```text
n = the number value of answer? + 1     # answer = "abc"  →  n = nothing (ikke crash)
v = the number value of "41"? + 1       #                  →  v = 42
say "{the text of maybe?}"              # maybe = nothing  →  "nothing"
```

Præcise regler (C03-kontrakten i ITERATION_PLAN §4.5):
1. **Hele-udtryksgift — én regel, sætningsformet:** hvis NOGEN del af et udtryk er
   `nothing`, og udtrykket bærer en `?`, er HELE udtrykket `nothing`.
2. **Markerens placering er fri:** `q? plus 1` og `q plus 1?` er SAMME udtryk.
   Ved parse fjernes alle `?`-markere, og hele det færdige træ pakkes præcis ét
   sted (`QuestionE`). Der findes aldrig indlejrede markere i det dumpede AST —
   kun én rod-pakning (fastlåst af golden 18-optional og kryds-skin-par).
3. **KUN fravær-af-værdi propagater** — nøjagtigt to throw-sites i fortolkeren:
   - regne-/rækkefølge-operationer på `nothing` (`plus/minus/times/divided/mod`,
     `gt/gte/lt/lte`),
   - felt-læsning fra `nothing` (`the text of maybe?` / `maybe.text?`).
   Out-of-bounds (`item 9 of xs`), ukendte felter på RIGTIGE things, ukendte
   funktioner og ugyldig json FEJLER stadig — også under `?`. `?` dækker aldrig
   logikfejl.
4. **Lighed med `nothing` er ALDRIG gift** — det ER testen:
   `if x is nothing then ...` / `if x is not nothing then ...` virker som altid,
   også når `x` er `nothing` (eq/ne er fritaget for regl 3).
5. **Uden `?` fejler det stadig højt** med den venlige sætning + fix-hint
   ("kan ikke regne med 'nothing' — tilføj '?' ... eller tjek værdien med
   'is nothing' først"). `NothingSignal` fanget af `try ... if it fails`? NEJ —
   det er fravær, ikke en fejl; kun `QuestionE`-grænsen opsluger signalet.
6. Resultatet testes med de eksisterende tjek fra regel 4; `say "{...}"` viser
   `nothing` som tekst. Gælder begge skins og streng-interpolation `{...}`.

## 3. Result<T,E>

```text
fn read_config(path: String) -> Result<Config, ConfigError> {
    text = File.read(path)?            # Err propagater til kalderen
    parsed = json.parse(text)?         # IoError → ConfigError: auto-konvertering via From-trait
    Config.from_json(parsed)
}
```

Kombinatorer:

```text
r.map(|v| v * 2)
r.and_then(validate)                   # flat_map
r.or_else(fallback_fn)
r.unwrap_or(default)  r.unwrap_or_else(gen)
r.expect("config var påkrævet")        # paniker med besked ved Err
r.ok()                                 # → Optional
result_tuple.unzip()
```

Fejl-typer: enhver type kan være E; stdlib bruger hierarkiet `Error` base + specifikke typer; auto-konvertering via `From<E2> for E1`.

## 4. ? operatoren — præcise regler

- `expr?` hvor expr: `Result<T,E>` → `T` i Ok-fald; `return Err(e.into())` ellers.
- `expr?` hvor expr: `T?` → `T`; `return none` ellers.
- Kun gyldigt i funktioner der returnerer kompatible `Result/_?` (eller i `try { }`-blokke).
- Chaining: `File.open(p)?.read()?.parse()?`.

## 5. Panic

```text
panic("umulig tilstand: {}", state)
assert(cond, "besked")          # debug-only
require(cond, "besked")         # altid aktiv
unreachable()                   # debug: panik; release: UB-markeret (lint)
todo("ikke implementeret")
```

Panics unwinder stacken (destructors/defer køre) indtil:
- procesgrænse (default): besked + backtrace + exit code 101
- nærmeste `catch`

## 6. try / catch / finally

```text
try {
    run_plugin(untrusted_input)
} catch e: PluginError {
    log.warn(e)
} catch e {                          # alle andre panics
    log.error(e.backtrace)
} finally {
    cleanup()
}
```

Brugsområder: plugin-grænseflader, FFI-grænser, top-level crash-handlers, benchmarks. Catch af `dynamic`-fejl ved dynamisk kode.

## 7. Backtraces og diagnostics

- Panics printer fil:linje + symboliseret backtrace (DWARF/PDB) i debug; `NOVA_BACKTRACE=full`.
- `Result.Err` kan bære `.trace` (capture valgfri, `--error-trace`).
- Strukturerede diagnostics fra compileren: kode (E0432-stil), span, notes, fixes (machine-applicable til LSP).

## 8. Interop

- C: errno/GSStatus → `Result` via bindings-generator.
- Python: exceptions → `Err(PyError)` ved grænsefladen.
- JVM: checked/unchecked exceptions → `Result<_, JavaThrowable>` i bridge-API.
