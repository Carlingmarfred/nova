# Nova Unikke Features — "kun Nova kan dette"

Status: **BESLUTTET** — del af sprogets identitet. Krav til hver feature:
1. Den gør noget ingen mainstream sprog kan (eller kun i fragmenteret/akademisk form).
2. Den læses som naturlige sætninger i Nova Natural.
3. Den bygger på kernen (IR, ARC, Flow-runtime) uden at ændre den.

Ærlighedsregel: hvor en idé findes et andet sted (forskning, niche-sprog), nævner vi det — Novas bidrag er **integrationen**: én mekanisme, naturlig syntax, hele vejen fra script til native.

---

## U1. Flow<T> — én samlingstypе for ALT der flyder

Lister, lazy generatorer, fillinjer, netværks-events, kanal-beskeder og signal-historik er **samme type**: `Flow<T>`. Ét ordforråd (~40 operationer) virker overalt.

```text
# Strøm en 10 GB log — bruger aldrig mere hukommelse end én linje:
repeat for each line in "huge.log"
    if the line contains "ERROR" then say it
done

the errors are every line of "huge.log" that contains "ERROR"   # lazy Flow
say how many items are in the errors                             # tæller ved behov

# Kanaler er også Flows — samme API som lister:
repeat for each message in the inbox
    handle message
done
```

**Hvorfor unikt:** Rust adskiller `Iterator`/`Stream` (+ async-farvede funktioner); JS adskiller iterable/observable; Python adskiller liste/generator/async-generator. Nova har **ét** API, synkront eller asynkront efter kontekst — compileren vælger implementering, brugeren skriver det samme.
Kerne: `Iterable`-traiettens generalisering; monomorphiseret → zero-cost. **M2.**

## U2. Table — kolonnetabellen er en primitiv

Pandas/numpy-følelse **i selve sproget**, ikke et bibliotek:

```text
sales is a table from "sales.csv"

say the columns of sales                          # dato | produkt | beløb
big is the rows of sales where beløb > 1000       # filter (SIMD-aggregering)
per-product is sales grouped by produkt summing beløb
top3 is per-product biggest-first take 3
enriched is sales joined-with prices matching product == name
```

- Kolonneorienteret hukommelse, vektoriserede aggregationer, zero-copy import af CSV/JSON.
- Eget filformat `.ntab` (kolonne-komprimeret, memory-mappable).
- **Query-pushdown:** peger tabellen på en database (std.database) genereres SQL fra de samme fraser — filter/groupby/sum udføres i databasen, ikke i Nova.
**Hvorfor unikt:** kun kdb+/q har tabeller som primitiver — men q er skrivebords-ekstoterisk. Nova bringer det til et general-purpose sprog med natural-syntax. **M5** (med nova-array).

## U3. Persistent collections + UNDO som sprog-service

Alle kerne-collections er persistent (strukturel deling) under motorhjelmen — mutation-API'erne føles almindelige. Enhver binding kan trackes:

```text
track the shopping-list                # nu gemmes alle versioner
add "mælk" to shopping-list
add "brød" to shopping-list

undo the last change to shopping-list  # → ["mælk"] igen
redo it                                # → ["mælk","brød"]
```

**Hvorfor unikt:** intet mainstream sprog giver undo/redo som sprog-feature. GUI-apps får undo gratis; nova-gui binder en tidslinje-slider direkte: `the slider position binds to a version of tasks`.
Pris: kun for trackede bindings (ARC-version-noder, delt struktur = billig). **M4.**

## U4. Temporale spørgsmål om variabler

Trackede bindings kan spørges om deres **fortid**:

```text
did the score ever go above 500
when did the score first reach 100                 # tidspunkt + version
how many times did the temperature fall            # antal fald mellem målinger
what was the temperature an hour ago
```

Én mekanisme driver fire ting: **undo (U3), temporale spørgsmål, time-travel-debuggeren (F1) og revisionslog/sporbarhed** (`nova audit` printer ændringshistorikken som tabel).
Signals (§21) tracker automatisk — `when the score changes` er allerede temporal syntaks. **Hvorfor unikt:** ingen andre sprog har variabel-historik som forespørgselssprog. **M4-M5.**

## U5. Tillids-sporing (data-taint) i typesystemet

Værdier fra utro kilder bærer usynlige stempler; følsomme "vaske" kræver rene værdier:

```text
name is ask "Navn: "                               # stemplet: ←tastatur
page is http.get(url).body_text()                  # stemplet: ←netværk

database.query("SELECT * WHERE navn = '{name}'")   # COMPILE-FEJL:
                                                   #   netto-stempel kan ikke nå db-sink

clean is sanitize(name)                            # ny, ren værdi (dokumenteret check)
database.query("SELECT * WHERE navn = '{clean}'")  # ok
```

- Stempler propagerer gennem operationer (sammenkædning bevarer oprindelse).
- Statiske sinks (db.query, File.write, process.run) erklærer accepterede tillidsniveauer.
- Compile-time når dataflow er statisk; runtime-stempler ved `dynamic`-grænser.
**Hvorfor unikt:** Perl havde runtime-taint pr. fil i 1993; ingen moderne sprog har typet, per-værdi, granulær taint integreret med capabilities (E2). Sammen udgør de: *programmet kan ikke gøre mere end sine rettigheder, og data kan ikke rejse længere end sin oprindelse tillader.* **M4.**

## U6. Naturlige forespørgsler — ét query-sprog over alt

```text
adults is the users where age is at least 18 sorted by last-name
counts  is orders grouped-by city counting rows
```

Samme fraser virker på: Array (in-memory, monomorphiseret), Table (vektoriseret), Flow (streamende), database-forbindelse (**SQL-pushdown**), CRDT-replica (se U13-note). Linq-idéen — men i naturlige sætninger og med pushdown til SQL *og* til `.ntab`. **M3 (arrays/db), M5 (tables).**

## U7. Tilstandsmaskiner som deklaration

```text
a TrafficLight is the states
    red    waits 30 seconds then becomes green
    green  waits 25 seconds then becomes yellow
    yellow waits 5 seconds  then becomes red
done

light is a new TrafficLight
advance light                       # udfører den aktuelle overgang (med wait-regler)
say light                           # "green"
```

- Compileren verificerer: alle tilstande opnåelige, ingen døde ender (uden `finishes`), exhaustive `check light state`-matches.
- Overgange kan bære guards og handlinger: `red waits 30 seconds when emergency then becomes flashing`.
- Brug: spil-AI, protokoller, UI-flow, ordre-livscyklusser — klassisk kilde til bugs der bliver umulige.
**Hvorfor unikt:** SCXML/Statecharts findes som værktøj; intet stort sprog har tilstandsmaskine-deklaration i kernen med exhaustiveness-checks. **M3.**

## U8. Eksakt matematik-blokke

```text
exact
    if 0.1 plus 0.2 is equal to 0.3 then say "matematik stemmer!"    # JA her
    price is 19.99 times 3                                            # eksakt decimal
done
```

Inde i blokken promoveres literals til `Rational`/`Decimal`; sammenligninger er eksakte. Bliver beregningen irrationel (`sqrt`), falder den bevidst tilbage til float med compiler-note. Undervisnings-guld og penge-sikkert. **M2.**

## U9. Deterministisk simulering indbygget

```text
nova test --sim seed=42 --speed=1000x
```

Scheduleren, random og tid er injicerbare; samme seed = samme eksekvering **inklusive tråde/tasks**. Race-bugs reproduccerer sig selv; fejl-loggen kan genafspilles step-by-step (bygger på F1 time-travel).
**Hvorfor unikt:** FoundationDB gør det internt med egen stack; ingenting leverer det som sprog-standard. **M4.**

## U10. @incremental — beregn kun hvad der ændrede sig

```text
@incremental
to typecheck with files          # genberegner KUN noder hvis inputs ændredes
    ...
done
```

Salsa/Adapton-inspireret memoisering på funktionsniveau med finkornet input-tracking. Novas egen compiler bruger den (dogfooding) — derfor er genkompilering efter én linjes ændring ofte < 50 ms. **M5.**

## U11. Tid som sætning

```text
every 5 seconds { ping }
every day at 09:00 { make-backup }
in 30 seconds { remind-me "pause nu" }
when the clock strikes friday 16:00 { say "weekend!" }
```

Scheduler-integration: scripts holder VM'en i live til timer udløber; services bruger samme syntax. Cron/systemd-timer-logik bliver læsbar kode. **M3.**

## U12. `nova why` — programmet forklarer sig selv

Ved breakpoint, crash eller `pause the program`:

```text
> why did we enter this branch
score var 512 (over grænsen 500) — sidst ændret af add 12 to score (linje 88),
som blev kaldt fra level-up() (linje 41). Historik: 480 → 512.

> what touched the config most recently
file watch (config.nova, 14:02:11)

> why is this loop slow
847 iterationer; 96% af tiden i contains() — overvej index_of på sorteret felt.
```

Læser U4-historien + effect-sporet. Ingen debugger-kompetence kræves — man stiller spørgsmål på engelsk. **M6.**

## U13. Pure-Nova stacken — alt implementeret selv

Novas stdlib afhænger af **intet undtagen OS-syscalls**:

```text
egen regex-motor (RE2-model, lineær tid)      egen JSON/TOML/CSV/CBOR
egen TLS-stack (pure-Nova, M4)                egen indlejret db ("nova-db", sqlite-API)
egen kompression (deflate/zstd-lite)          eget billedformat-sæt (png/jpeg decode)
egen .ntab kolonneformat                      egen unicode-kollationstabell-generator
libc valgfri på Linux (direkte syscalls)      Windows: ren WinAPI
```

Konsekvenser: cross-compiling uden sysroot-helvede; minimal supply-chain; `--runtime minimal` på < 100 KB er troværdig; sikkerheds-audit af ÉN kodebase. Dette er bogstaveligt "implementér sin egen datastruktur og alt hvad der kræves". *(Zigs kultur, men fuldt ud ført igennem inkl. TLS og DB.)* **Løbende, M1-M5.**

## U14. Grammatik-literals — skriv format, få parser

```text
the ini-format is the grammar
    file   = section*
    section= "[" name "]" newline pair*
    pair   = key "=" value newline
done

settings is the ini-format parsed from "config.ini"
```

Compileren genererer rekursiv descent-parser + AST-typer compile-time (PEG-semantik, venstre-rekursion detekteres). Nogen definerer et tekstformat → parseren er færdig. Kompletterer regex (flade mønstre) og JSON (kendte formater). **M6.**

---

## Ærlig sammenligning

| Feature | Nova | Python | Rust | JS/TS | Java | Swift | q/kdb |
|---|---|---|---|---|---|---|---|
| U1 Ét Flow-API sync+async | Ja | Nej (4 varianter) | Delvis (Iterator/Stream) | Nej | Nej | Nej | — |
| U2 Tabel-primitiv | Ja | Bibliotek | Bibliotek | Bibliotek | Bibliotek | Bibliotek | **Ja** |
| U3 Undo som sprog-feature | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U4 Variabel-historik-forespørgsler | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U5 Typet taint-tracking | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U6 Query-fraser m/ SQL-pushdown | Delvist unik (Linq-agtig, natural) | Nej | Nej | Delvis (LinQ i C#) | Nej | Nej | Delvis |
| U7 Tilstandsmaskiner i kernen | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U8 Eksakte matematik-blokke | Ja | Delvis (fractions manuelt) | Nej | Nej | BigDecimal manuelt | Decimal manuelt | — |
| U9 Deterministisk sim-test standard | Ja | Nej | Delvist (loom/externelt) | Nej | Nej | Nej | — |
| U10 @incremental i sproget | Ja | Nej | Biblioteker | Nej | Nej | Nej | — |
| U13 Pure self-hosted stdlib | Ja | C-bundet | Delvis | Runtime-bundet | JVM-bundet | C-bundet | C-bundet |

Konklusionen er ikke "vi opfandt alle idéerne" — det er **at kombinationen, integrationen og natural-syntaxen er unik**: historik-motoren driver undo + debugging + revision; Flow motiverer iteratorer + streams + kanaler; taint + capabilities giver end-to-end sikkerhed.

## Milepæle

| Feature | M |
|---|---|
| U8 exact-blokke | M2 |
| U1 Flow | M2 |
| U11 Tid-udtryk | M3 |
| U6 Queries (array/db) | M3 |
| U7 Tilstandsmaskiner | M3 |
| U9 Sim-test | M4 |
| U3 Undo / U4 Historik | M4-M5 |
| U5 Taint | M4 |
| U10 @incremental | M5 |
| U2 Table | M5 |
| U12 nova why | M6 |
| U14 Grammatik-literals | M6 |
