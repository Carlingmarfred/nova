# Nova Unikke Features — "kun Nova kan dette"

Status: **BESLUTTET** — del af sprogets identitet. Krav til hver feature:
1. It does something no mainstream language can (or only in fragmented/academic form).
2. It reads as natural sentences in Nova Natural.
3. It builds on the kernel (IR, ARC, Flow runtime) without changing it.

Honesty rule: where an idea exists elsewhere (research, niche languages), we say so — Nova's contribution is **the integration**: one mechanism, natural syntax, all the way from script to native.

---

## U1. Flow<T> — én samlingstypе for ALT der flyder

Lists, lazy generators, file lines, network events, channel messages and signal history are **the same type**: `Flow<T>`. One vocabulary (~40 operations) works everywhere.

```text
# Stream a 10 GB log — never uses more memory than one line:
repeat for each line in "huge.log"
    if the line contains "ERROR" then say it
done

the errors are every line of "huge.log" that contains "ERROR"   # lazy Flow
say how many items are in the errors                             # counts on demand

# Channels are Flows too — same API as lists:
repeat for each message in the inbox
    handle message
done
```

**Why unique:** Rust separates `Iterator`/`Stream` (+ async-colored functions); JS separates iterable/observable; Python separates list/generator/async-generator. Nova has **one** API, synchronous or asynchronous by context — the compiler picks the implementation, the user writes the same thing.
Kerne: `Iterable`-traiettens generalisering; monomorphiseret → zero-cost. **M2.**

## U2. Table — kolonnetabellen er en primitiv

A pandas/numpy feel **in the language itself**, not a library:

```text
sales is a table from "sales.csv"

say the columns of sales                          # date | product | amount
big is the rows of sales where amount > 1000       # filter (SIMD aggregation)
per-product is sales grouped by product summing amount
top3 is per-product biggest-first take 3
enriched is sales joined-with prices matching product == name
```

- Kolonneorienteret hukommelse, vektoriserede aggregationer, zero-copy import af CSV/JSON.
- Eget filformat `.ntab` (kolonne-komprimeret, memory-mappable).
- **Query pushdown:** point the table at a database (std.database) and SQL is generated from the same phrases — filter/groupby/sum run in the database, not in Nova.
**Hvorfor unikt:** kun kdb+/q har tabeller som primitiver — men q er skrivebords-ekstoterisk. Nova bringer det til et general-purpose sprog med natural-syntax. **M5** (med nova-array).

## U3. Persistent collections + UNDO som sprog-service

All core collections are persistent (structural sharing) under the hood — the mutation APIs feel ordinary. Any binding can be tracked:

```text
track the shopping-list                # nu gemmes alle versioner
add "milk" to shopping-list
add "bread" to shopping-list

undo the last change to shopping-list  # → ["milk"] again
redo it                                # → ["milk","bread"]
```

**Why unique:** no mainstream language gives undo/redo as a language feature. GUI apps get undo for free; nova-gui binds a timeline slider directly: `the slider position binds to a version of tasks`.
Pris: kun for trackede bindings (ARC-version-noder, delt struktur = billig). **M4.**

## U4. Temporal queries about variables

Tracked bindings can be queried about their **past**:

```text
did the score ever go above 500
when did the score first reach 100                 # tidspunkt + version
how many times did the temperature fall            # number of drops between readings
what was the temperature an hour ago
```

One mechanism drives four things: **undo (U3), temporal queries, the time-travel debugger (F1) and revision log/traceability** (`nova audit` prints the change history as a table).
Signals (§21) track automatically — `when the score changes` is already temporal syntax. **Why unique:** no other language has variable history as a query language. **M4-M5.**

## U5. Tillids-sporing (data-taint) i typesystemet

Values from untrusted sources carry invisible stamps; sensitive "sinks" require clean values:

```text
name is ask "Navn: "                               # stemplet: ←tastatur
page is http.get(url).body_text()                  # stamped: ←network

database.query("SELECT * WHERE navn = '{name}'")   # COMPILE-FEJL:
                                                   #   the network stamp cannot reach the db sink

clean is sanitize(name)                            # new, clean value (documented check)
database.query("SELECT * WHERE navn = '{clean}'")  # ok
```

- Stamps propagate through operations (concatenation preserves origin).
- Static sinks (db.query, File.write, process.run) declare accepted trust levels.
- Compile time when dataflow is static; runtime stamps at `dynamic` boundaries.
**Why unique:** Perl had per-file runtime taint in 1993; no modern language has typed, per-value, fine-grained taint integrated with capabilities (E2). Together they mean: *the program cannot do more than its permissions, and data cannot travel further than its origin allows.* **M4.**

## U6. Natural queries — one query language everywhere

```text
adults is the users where age is at least 18 sorted by last-name
counts  is orders grouped-by city counting rows
```

The same phrases work on: Arrays (in-memory, monomorphized), Tables (vectorized), Flows (streaming), database connections (**SQL pushdown**), CRDT replicas (see U13 note). The Linq idea — but in natural sentences and with pushdown to SQL *and* `.ntab`. **M3 (arrays/db), M5 (tables).**

## U7. Tilstandsmaskiner som deklaration

```text
a TrafficLight is the states
    red    waits 30 seconds then becomes green
    green  waits 25 seconds then becomes yellow
    yellow waits 5 seconds  then becomes red
done

light is a new TrafficLight
advance light                       # performs the current transition (with wait rules)
say light                           # "green"
```

- The compiler verifies: every state reachable, no dead ends (without `finishes`), exhaustive `check light state` matches.
- Transitions can carry guards and actions: `red waits 30 seconds when emergency then becomes flashing`.
- Brug: spil-AI, protokoller, UI-flow, ordre-livscyklusser — klassisk kilde til bugs der bliver umulige.
**Why unique:** SCXML/Statecharts exist as tools; no major language has state-machine declaration in the kernel with exhaustiveness checks. **M3.**

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

Scheduler, random and time are injectable; same seed = same execution **including threads/tasks**. Race bugs reproduce themselves; the error log can be replayed step-by-step (builds on F1 time travel).
**Why unique:** FoundationDB does it internally with its own stack; nothing ships it as a language standard. **M4.**

## U10. @incremental — compute only what changed

```text
@incremental
to typecheck with files          # recomputes ONLY nodes whose inputs changed
    ...
done
```

Salsa/Adapton-inspired function-level memoization with fine-grained input tracking. Nova's own compiler uses it (dogfooding) — which is why recompiling after a one-line change is often < 50 ms. **M5.**

## U11. Time as a sentence

```text
every 5 seconds { ping }
every day at 09:00 { make-backup }
in 30 seconds { remind-me "pause nu" }
when the clock strikes friday 16:00 { say "weekend!" }
```

Scheduler integration: scripts keep the VM alive until timers fire; services use the same syntax. Cron/systemd timer logic becomes readable code. **M3.**

## U12. `nova why` — programmet forklarer sig selv

Ved breakpoint, crash eller `pause the program`:

```text
> why did we enter this branch
score was 512 (over the limit 500) — last changed by add 12 to score (line 88),
som blev kaldt fra level-up() (linje 41). Historik: 480 → 512.

> what touched the config most recently
file watch (config.nova, 14:02:11)

> why is this loop slow
847 iterations; 96% of the time in contains() — consider index_of on a sorted field.
```

Reads the U4 history + the effect trace. No debugger expertise required — you ask questions in English. **M6.**

## U13. The pure-Nova stack — everything implemented in-house

Nova's stdlib depends on **nothing but OS syscalls**:

```text
own regex engine (RE2 model, linear time)      own JSON/TOML/CSV/CBOR
egen TLS-stack (pure-Nova, M4)                egen indlejret db ("nova-db", sqlite-API)
own compression (deflate/zstd-lite)            own image format set (png/jpeg decode)
egen .ntab kolonneformat                      egen unicode-kollationstabell-generator
libc optional on Linux (direct syscalls)       Windows: pure WinAPI
```

Consequences: cross-compiling without sysroot hell; minimal supply chain; `--runtime minimal` under 100 KB is credible; security audit of ONE codebase. This is literally "implement your own data structures and everything they require". *(Zig's culture, but carried all the way through incl. TLS and DB.)* **Ongoing, M1-M5.**

## U14. Grammar literals — write the format, get a parser

```text
the ini-format is the grammar
    file   = section*
    section= "[" name "]" newline pair*
    pair   = key "=" value newline
done

settings is the ini-format parsed from "config.ini"
```

The compiler generates a recursive-descent parser + AST types at compile time (PEG semantics, left recursion detected). Someone defines a text format → the parser is done. Complements regex (flat patterns) and JSON (known formats). **M6.**

---

## Honest comparison

| Feature | Nova | Python | Rust | JS/TS | Java | Swift | q/kdb |
|---|---|---|---|---|---|---|---|
| U1 Ét Flow-API sync+async | Ja | Nej (4 varianter) | Delvis (Iterator/Stream) | Nej | Nej | Nej | — |
| U2 Tabel-primitiv | Ja | Bibliotek | Bibliotek | Bibliotek | Bibliotek | Bibliotek | **Ja** |
| U3 Undo som sprog-feature | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U4 Variable-history queries | Yes | No | No | No | No | No | No |
| U5 Typet taint-tracking | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U6 Query-fraser m/ SQL-pushdown | Delvist unik (Linq-agtig, natural) | Nej | Nej | Delvis (LinQ i C#) | Nej | Nej | Delvis |
| U7 Tilstandsmaskiner i kernen | Ja | Nej | Nej | Nej | Nej | Nej | Nej |
| U8 Eksakte matematik-blokke | Ja | Delvis (fractions manuelt) | Nej | Nej | BigDecimal manuelt | Decimal manuelt | — |
| U9 Deterministisk sim-test standard | Ja | Nej | Delvist (loom/externelt) | Nej | Nej | Nej | — |
| U10 @incremental i sproget | Ja | Nej | Biblioteker | Nej | Nej | Nej | — |
| U13 Pure self-hosted stdlib | Ja | C-bundet | Delvis | Runtime-bundet | JVM-bundet | C-bundet | C-bundet |

Konklusionen er ikke "vi opfandt alle idéerne" — det er **at kombinationen, integrationen og natural-syntaxen er unik**: historik-motoren driver undo + debugging + revision; Flow motiverer iteratorer + streams + kanaler; taint + capabilities giver end-to-end sikkerhed.

## Milestones

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
