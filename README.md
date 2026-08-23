# Nova

Et general-purpose programmeringssprog: **C++ performance + Python enkelhed + Java økosystem** — bygget om én stærk kerne (compiler + typesystem + IR + runtime) med alle andre features ovenpå.

> **Status: v0.12-bootstrap** — Python-fortolkeren i `bootstrap/` er grøn på hele end-to-end testsuiten (`python tests/run_tests.py`, 191 tests inkl. begge eksempler). **G0-gaten er lukket.** Kompakt shorthand-skin, Optional/`?`, ægte moduler med navnerum og stdlib v0 (`use the standard X library`: json/file/random/time/math/text/list) er implementeret. Se [project-notes.md](project-notes.md) §5.

```
# Nova Natural (primær syntax — læses som sætninger):
when the program starts
    secret is a random number between 1 and 100
    repeat until the guess is the secret
        answer is ask "Dit gæt: "
        if answer is not a number then say "Det er ikke et tal."
        otherwise if guess is less than secret then say "Højere!"
        otherwise if guess is greater than secret then say "Lavere!"
        otherwise
            say "Rigtigt! Tallet var {secret}."
            stop the loop
        done
    done
done

# Kompakt shorthand (samme AST — ekspert-stenografi; planlagt, ikke i bootstrap endnu):
x = 10                          # inference → i32, mutable binding
nums = [1, 2, 3].map(x => x * 2).filter(x => x > 4)
fn read(path: String) -> Result<String, IoError> { Ok(File.read(path)?) }
```

## Dokumentation

| Dokument | Indhold |
|---|---|
| [docs/ITERATION_PLAN.md](docs/ITERATION_PLAN.md) | **Levende plan: workflow, prioriteter, fasegates, 1.0-kriterier, status/changelog — altid opdateret** |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Compiler-pipeline, Nova IR, backends, runtime, VM, tooling |
| [specs/natural_syntax.md](specs/natural_syntax.md) | **Nova Natural: den engelske sætnings-syntax** — fuld ordforrådstabel + desugaring |
| [specs/language_reference.md](specs/language_reference.md) | Komplet sprogreference: alle constructs, operatorer, collections |
| [specs/syntax/grammar.md](specs/syntax/grammar.md) | Formel EBNF-grammatik |
| [specs/syntax/lexical.md](specs/syntax/lexical.md) | Tokens, literals, keywords, operatorer |
| [specs/type_system.md](specs/type_system.md) | Statisk/dynamisk typing, inference, generics, traits, unions |
| [specs/memory_model.md](specs/memory_model.md) | ARC-standard, ownership, unsafe, GC-profiler |
| [specs/error_handling.md](specs/error_handling.md) | Result/Optional/`?`, panics, exceptions |
| [specs/concurrency.md](specs/concurrency.md) | async/await, parallel, channels, select, structured concurrency |
| [specs/metaprogramming.md](specs/metaprogramming.md) | Compile-time eval, derive-macros, reflection |
| [specs/module_system.md](specs/module_system.md) | Moduler, packages, `project.nova`, dependency-håndtering |
| [specs/standard_library.md](specs/standard_library.md) | Komplet stdlib-overflade med Python-paritetstabel |
| [examples/tour.nova](examples/tour.nova) | Hele sproget i ét kommenteret program |
| [docs/EXTENSIONS.md](docs/EXTENSIONS.md) | 16 foreslåede udvidelser: units, signals, actors, contracts, time-travel debug m.m. |
| [specs/unique_features.md](specs/unique_features.md) | **De 13 unikke features** der kun Nova kan: Flow, Table, undo/historik, taint, tilstandsmaskiner m.fl. |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Milepæle M0–M8 |

## Beslutningslog (open questions besvaret)

| Spørgsmål | Beslutning | Begrundelse |
|---|---|---|
| Memory model | **ARC som default** + compile-time escape analysis (fjerner retain/release hvor muligt). `owned` opt-in, `unsafe` raw pointers. GC kun via `--runtime full`. Cykler håndteres af cycle collector i full-runtime; i minimal kræves `weak`. | Deterministisk deallokering, kan implementeres som IR-pass tidligt, Swift har bevist modellen. Fuld GC udskydes uden at blokere frontend. |
| Dynamic typing | `dynamic` = komplet runtime-system (tagged value + inline caches i VM / vtable native). Specialisering er en senere optimering (typeprofilering fra VM → guarded fast paths), ikke forudsætning. | Kernen skal fungere 100% dynamisk først; specialisering er additivt. |
| C++ interop | **C ABI først** + automatisk header-import for C. Ægte C++ interop via genererede shims, senere. ABI antager aldrig C++-semantik (ingen exceptions/RTTI i ABI'et). | Fuldt C++ interop er et projekt på størrelse med selve compileren. |
| Java interop | Ingen JVM-backend i første version. Bridge via JNI-lignende grænseflade gennem C ABI. JVM-backend genovervejes når Nova IR er stabil. | En JVM-backend låser IR-designet til Javas typesystem (ingen unsigned, ingen værdityper). |
| Runtime | To profiler fra dag 1: `--runtime minimal` (ARC-core, ingen scheduler/reflection-data) og `--runtime full`. Stdlib-core kalder aldrig runtime direkte, kun gennem capability-interfacers. | Tvinger ren lagdeling; gør embedded/single-file builds mulige. |
| Concurrency | `parallel` = compiler-styret (work-stealing task scheduler), plus eksplicitte primitiver (`spawn`, `Channel`, `Mutex`, `select`) til fuld kontrol. Structured concurrency som grundprincip. | De to niveauer er komplementære; `parallel` er syntakt sukker over task-systemet. |
| Syntax | **Nova Natural som primær syntax**: almindelige engelske ord i faste fraser — `say`, `ask ... and remember it as`, `set x to`, `if ... then ... otherwise ... done`, `repeat until ... done`, `to greet with name ... done`. Blokke termineres med `done` (éntydigt, ingen indentation-følsomhed). Kompakt shorthand (`{}`, `=>`, symbol-operatorer, `x = 10`) forbliver gyldig ekspert-form — **begge skins kompilerer til identisk AST**. Eksplicit typer påkrævet på offentlige API-signaturer (kompakt form). | Brugerens kernekrav: kode skal kunne skrives ord for ord som man ville forklare idéen til et menneske. `done`-terminatorer undgår JS-agtig ASI-fejlsource og Python-indentation-skørehed; to-skins-designet bevarer fuld udtrykskraft uden at duplikere semantik. |
| Extensions | **Alle 16 udvidelser godkendt**: refinement types, units (dimensional analysis), verificerede format-strenge, pipelines/`then`, signals, actors, contracts, capability-tilladelser, reproducible builds, time-travel debugger, notebook/literate mode, `nova explain`, API-diff, undervisningspakke + blok-editor, embedding-API, native hot reload. Integreret i kernens specs — se [docs/EXTENSIONS.md](docs/EXTENSIONS.md). | Udvider sprogets dækning fra "kan alt" til "har det bedste værktøj til det"; alle bygger ovenpå kernen uden at ændre den. |
| Unikke features | **13 features der gør Nova unik** (se [specs/unique_features.md](specs/unique_features.md)): Flow<T> (ét API for lister/streams/kanaler), Table som sprog-primitiv med SQL-pushdown, undo/redo + variabel-historik-forespørgsler (`track`/`undo`/`ever`), typet tillids-sporing (taint), tilstandsmaskiner i kernen, `exact`-matematik-blokke, deterministisk sim-test standard, `@incremental`, tidsudtryk (`every day at 09:00`), `nova why`, grammatik-literals og **pure-Nova stacken** (stdlib afhænger kun af OS-syscalls — egen regex/TLS/db/kompression). | Brugerens krav: features ingen andre sprog har — uden at gå på kompromis med læsbarhed. Ærlig sammenligningstabel pr. feature ligger i spec'en; unikheden ligger i integrationen: én historik-motor driver undo + debugging + revision, én Flow-motor driver iteratorer + streams + kanaler. |

### Yderligere fastlagte kernedecisioner

- **String**: UTF-8, immutable `String` + `StringBuilder`; `char` = Unicode scalar value.
- **Heltal**: fixed-width default (`i32` inferres for int-literals); overflow-check i debug, wrapping i release (opt-in `@checked`). `BigInt` i stdlib.
- **Floats**: IEEE-754 `f32`/`f64`; `Decimal` og `Rational` i stdlib.
- **Error model**: `Result<T,E>`/`Optional<T>` primær; `throw`/`try-catch` = panic-handler til undtagelsestilfælde, aldrig kontrolflow.
- **Contracts (v0.11)**: `requires` evalueres eager ved kald; `ensures` udskydes til funktionsafslutning og evalueres i funktions-lokalt scope — post-betingelser kan referere sluttilstanden. Fejl → `NovaError`, fangbar med `try ... if it fails`.
- **Type unions**: anonyme sumtyper tilladt: `i32 | String`.
- **Fil-endelse**: `.nova`.

## Kerneprincip

> Nova implementerer ikke Python, Java og C++ som tre systemer. Én kerne — compiler, typesystem, IR, runtime — og derefter Python-dynamik, Java-reflection og C++-kontrol ovenpå samme kerne.
