# Nova Extensions — BESLUTTEDE udvidelser v1

Status: **Alle 16 features godkendt og integreret i kernespecifikationerne.**

| Feature | Hjemmel i specs |
|---|---|
| A1 Refinement types | type_system.md §11 |
| A2 Units | type_system.md §12 + standard_library.md §21 |
| A3 Verificerede format-strenge | language_reference.md §7 (strings) — compiler-pass |
| B1 Pipelines / `then` | language_reference.md §20 |
| C1 Signals | language_reference.md §21 |
| D1 Actors | concurrency.md §7 |
| E1 Contracts | language_reference.md §23 |
| E2 Capability-tilladelser | module_system.md (project.nova `[permissions]`) |
| E3 Reproducible builds | module_system.md §4 (build-flag) + registry |
| F1 Time-travel debugger | docs/ARCHITECTURE.md §8 tooling |
| F2 Notebook/literate | ARCHITECTURE.md §8 (`nova notebook`, `.nova.md`) |
| F3 `nova explain` | diagnostics-spec (fejl-ID + offline vidensbase) |
| F4 API-diff | `nova api-diff` CLI |
| F5 Undervisningspakke + Blocks | `nova trace`, blok-editor → natural-eksport |
| G1 Embedding-API | runtime/minimal-profil + libnova_vm.h |
| G2 Native hot reload | dev-byggets dynamiske biblioteker |

Nedenfor: oprindelige motivationer og eksempler (beholdt som design-rationale).

---

## Tema A — Typer der læser sig selv og fanger flere fejl

### A1. Refinement types (betingede typer)

Typer med indbyggede regler — tjekkes ved grænsefladen, optimeret væk indeni.

```text
# Natural
an age is a whole number from 0 to 130
a positive is a number greater than 0

to buy-ticket with age: age
    # compileren GARANTERER her at age er gyldig
done
```

```text
# Kompakt
type Age = i32 where self >= 0 && self <= 130
type NonEmpty<T> = Array<T> where self.len() > 0

fn buy_ticket(age: Age) { ... }
buy_ticket(user.age)              # runtime-check ved kaldsgrænsen
buy_ticket(-5)                    # COMPILE-fejl hvis konstant, ellers runtime-check
```

Implementering: subtype af basistype + predicate; SMT-light-verificering af konstante argumenter, ellers automatisk grænse-check. Zero-cost inde i funktionen når flow-analysen kan bevise predikatet.
**Hvorfor:** fjerner en hel klasse validerings-boilerplate; perfekt match med natural-syntaxens læsbarhed. Risiko: lav-middel. **Milestone: M4.**

### A2. Enheder og dimensioner (units of measurement)

```text
# Natural
the distance is 100 meters
the time was 9.58 seconds
the speed is the distance divided by the time       # m/s infereret
say "{the speed in kilometers per hour}"

the distance plus the time                          # COMPILE-FEJL: meter + sekund
```

```text
# Kompakt
let d = 100.m
let t = 9.58.s
let v = d / t            # Unit<Length/Time> — dimensionsanalyse i typesystemet
v.to::<km/h>()
```

Implementering: generisk `Unit<L,M,T,...>` med heltals-eksponenter; monomorphiserer til rå floats — **nul runtime-omkostning**. SI-enheder + præfikser i stdlib; valuta baseret på `Decimal`.
**Hvorfor:** Nova vil erstatte Python i science-computing — NASA's Mars Climate Orbiter-tab (~$327M) var et enhedsfejl. Risiko: middel (typesystem-arbejde). **Milestone: M5 (med nova-array).**

### A3. Compile-time verificerede format-strenge

`"{pris:.2} kr"` verificeres mod argumenttyper ved kompilering: forkert specifier (`{navn:.2}` på String) eller manglende variabel = compile-fejl, ikke runtime-crash. Gælder `say`, `format`, logning. Regex-literals er allerede compile-checkede — dette fuldender mønstret.
Risiko: lav. **Milestone: M2.**

---

## Tema B — Nye udtryksformer

### B1. Pipelines (`|>` og `then`-kæder)

```text
# Kompakt
contents |> split_lines |> filter(l => l.len() > 0) |> sorted() |> join("\n") |> print
```

```text
# Natural — taler præcis som man tænker:
take the file contents
    then split it by lines
    then keep the ones that are not empty
    then sort them
    then say the result
done
```

Desugares til almindelige metode-kald (zero-cost). `then` er reserveret ord kun i pipeline-kontekst. **Milestone: M2.**

---

## Tema C — Reaktivitet (GUI/games uden boilerplate)

### C1. Signals — automatiske afledte værdier

```text
# Natural
the score is a signal starting at 0
the rank is when the score changes: "Niveau {floor(score / 100)}"

when the score changes
    say "{score} → {rank}"
done

add 50 to the score          # → automatisk: "50 → Niveau 0"
add 60 to the score          # → "110 → Niveau 1" (rank opdaterede sig selv)
```

```text
# Kompakt
let score = signal(0)
let rank = computed(() => "Niveau {score.value / 100}")
effect { print("{score.value} → {rank.value}") }
score.value += 60
```

Pull-baseret, glitch-fri (topologisk invalidering) — samme model som SolidJS/SwiftUI `@Observable`. Bliver fundamentet under nova-gui: `the button text binds to the label` — UI der bare *er* sin tilstand. **Hvorfor:** GUI-spillet er i M5; signals skal være i kernen først, ikke bagklogt boltet på. Risiko: middel. **Milestone: M4 (kerne) → M5 (GUI-integration).**

---

## Tema D — Concurrency-udvidelser

### D1. Actors — isolerede tilstande med besked-protokol

```text
a BankAccount is an actor keeping
    a balance of 0

    on deposit with amount
        add amount to my balance
    on withdraw with amount
        if my balance is at least amount then
            take amount from my balance
            reply with "ok"
        otherwise
            reply with "ikke nok penge"
    done
done

account is a new BankAccount
send account "deposit" with 100
answer is ask account to "withdraw" with 150     # request/response, venter på svar
```

Én besked ad gangen pr. actor → ingen locks, ingen dataracer mulige. Implementeres ovenpå eksisterende tasks + channels (ingen ny runtime). Godt fit til spil-entities, servere og distribuerede systemer senere. **Milestone: M4.**

---

## Tema E — Robusthed og sikkerhed

### E1. Contracts (krav og løfter)

```text
to withdraw with amount
    requires amount is greater than 0
    requires amount is at most my balance
    ensures my balance is at least 0

    take amount from my balance
done
```

Kompakt: `@requires(x > 0) @ensures(result >= 0)`. Tjekkes i debug/tests; strippes i release (eller beholdes via profil). Samme mekanisme driver: fuzzing-generatorene (kontrakterne ER test-input-reglerne), dokumentationen (vises i hover/doc) og fremtidig letvægts-formel verificering. **Milestone: M3.**

### E2. Capability-tilladelser for scripts og pakker

```text
# project.nova
[permissions]
read  = ["data/*"]
write = false
network = ["api.example.com"]
spawn  = false
```

- Scripts/pakker skal **erklære** hvad de har brug for (som app-rettigheder på telefonen).
- Runtime håndhæver det; `nova install` viser tilladelses-dialog.
- Malicious-supply-chain angreb begrænses fra "fuld maskine" til "erklæret scope".
Bygger på den eksisterende VM-sandbox; native builds får den via runtime-hook på fs/net/process-API'erne. **Milestone: M4.**

### E3. Reproducible builds + signeret provenance

- `nova build --reproducible`: byte-identisk output fra samme source + compiler-version (frossen stdlib, deterministisk kodegen).
- Registry gemmer build-manifest + signatur; `nova verify <pkg>` efterprøver kæden.
**Hvorfor:** økosystem-tillid (XZ-utils-lektionen). **Milestone: M4-M5.**

---

## Tema F — Tooling der ingen konkurrent har

### F1. Time-travel debugger

VM'en optager eksekvering (variabel-historie i ringbuffere). `nova replay` lader dig **spole tilbage**:

```text
BREAK ved tour.nova:42 (gang 847 af 1000)
  guess = 62   secret = 62   tries = 7
[← tilbage] [frem →] [hvor kom 'tries' fra?] [watch: secret]
```

"Hvor fik denne variabel sin værdi?" = omvendt dataflow-søgning. Native-byggets instrumenteringstilstand giver det samme (langsommere). **Hvorfor:** beginner-venligt OG pro-værktøj; RR/time-travel findes i C++-verdenen men er sjældent og svært — her er det standard. Risiko: høj kompleksitet, men VM'en ejer allerede hele eksekveringen. **Milestone: M5-M6.**

### F2. Notebook + literate mode

- `nova notebook` → lokal web-kernel (Jupyter-protokol): celler, grafer inline (nova-plot), markdown mellem koden.
- `.nova.md`-filer: ```nova-fences udføres af `nova test --doc` — dokumentation der ALDRIG bliver forældet, fordi den testes.
**Milestone: M4 (kernel), M6 (plot-integration).**

### F3. `nova explain` — fejlmeddelelser med underviser

```text
error[E2031]: du sammenlignede en tekst med et tal
  --> guessing.nova:12:24
   |
12 |     if answer is less than secret then
   |         ------          ------
   |         tekst ('"75"')  tal (i32)
   |
help: konverter først:  set guess to the number value of answer
note: hvorfor: tekster sammenlignes bogstav for bogstav ("10" < "9"!)
```

Hver diagnostik har stabilt ID + offline vidensbase (`nova explain E2031`). Ingen cloud-afhængighed. **Milestone: løbende fra M1.**

### F4. API-diff og semver-politi

`nova api-diff v1.2.0..HEAD` → liste af tilføjede/fjernede/ændrede offentlige symboler + semver-dom: *"breaking: Parameter `timeout` fik default-værdi fjernet → kræver major bump"*. CI-gate for pakker. **Milestone: M5.**

### F5. Undervisningspakken

- `nova trace fil.nova` — Python-Tutor-agtig linje-for-linje-visualisering af alle variabler (terminal + web).
- **Nova Blocks**: Scratch-agtig blok-editor der **eksporterer til Nova Natural-tekst** (én vej — ingen lock-in). Natural-syntaxen er designet til netop dette: hver blok = én sætning.
**Milestone: M6+.**

---

## Tema G — Platform-rækkevidde

### G1. Embedding-API (Novas Lua-niche)

`libnova_vm.h` / `libnova.so`: host-applikationer (C/C++/C#/Rust/spilmotorer) indlejrer Nova-VM'en som scriptlag:

```c
nova_vm* vm = nova_new(&(nova_opts){ .permissions = NOVA_CAP_NONE });
nova_reg(vm, "move_player", host_move_player);
nova_run(vm, nova_readfile("level1.nova"));
```

Minimal-profilen (< 100 KB, statisk, ingen GC) er allerede designet til netop dette. Mod-scrip­ting, plugin-systemer, konfiguration-som-kode. **Milestone: M4.**

### G2. Hot reload også til native dev-builds

VM-mode har hot reload fra dag 1. Dev-native-tilstand bygger til dynamiske biblioteker og swapper ved safe-points (game-engine-mønstret). Ikke garanteret for alle programmer — compileren rapporterer præcis hvornår et reload kræver genstart. **Milestone: M6.**

---

## Prioriteret oversigt

| # | Feature | Værdi | Omkostning | Milepæl |
|---|---|---|---|---|
| 1 | A3 Format-strenge verificeres compile-time | Høj | Lav | M2 |
| 2 | B1 Pipelines / `then` | Høj | Lav | M2 |
| 3 | F3 `nova explain` | Høj | Lav | M1+ |
| 4 | E1 Contracts | Høj | Mellem | M3 |
| 5 | C1 Signals | Meget høj (GUI-spillets fundament) | Mellem | M4 |
| 6 | D1 Actors | Høj | Mellem | M4 |
| 7 | E2 Capabilities/sandbox | Meget høj (sikkerhed) | Mellem | M4 |
| 8 | G1 Embedding-API | Høj (ny målgruppe) | Mellem | M4 |
| 9 | F2 Notebook/literate | Høj | Mellem | M4 |
| 10 | A1 Refinement types | Mellem-høj | Mellem | M4 |
| 11 | E3 Reproducible builds | Mellem-høj | Mellem | M4-M5 |
| 12 | A2 Units | Meget høj for science | Høj | M5 |
| 13 | F1 Time-travel debug | Meget høj (differentiator) | Høj | M5-M6 |
| 14 | F4 API-diff | Mellem | Lav-mellem | M5 |
| 15 | F5 Undervisningspakke + Blocks | Høj strategisk | Høj | M6+ |
| 16 | G2 Native hot reload | Mellem | Høj | M6 |
