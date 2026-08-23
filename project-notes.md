# Nova — Project Notes

> **Status v0.13-bootstrap:** G0-gaten LUKKET; G1 i gang. B05/B01/B02 + C01/C02/B04
> + C03/C05 + B03+C06+C07+C08 stdlib v0 + docs-audit + C09 REPL gennemført;
> 199/199 end-to-end tests grønne (`python tests/run_tests.py`). Se changelog i §5.

## 1. Context & Goals

Building **Nova**, a new general-purpose programming language: C++ performance + Python
simplicity + Java ecosystem. Session progressed through: full design spec suite → natural
English syntax pivot → 16 extensions approved → 13 "unique-only-to-Nova" features approved
→ **implementation started** (Python bootstrap interpreter, `bootstrap/`).

Repo layout (working dir `oxtest/`):

- `AGENTS.md` — working agreement for agent sessions (read first)
- `README.md` — overview + full decision log
- `docs/` — ITERATION_PLAN.md (living plan, always updated), ARCHITECTURE.md,
  ROADMAP.md (M0–M6), EXTENSIONS.md (16 approved)
- `specs/` — language_reference, natural_syntax, type_system, memory_model,
  error_handling, concurrency, metaprogramming, module_system, standard_library,
  unique_features, syntax/grammar.md, syntax/lexical.md
- `examples/` — tour.nova, guessing_game.nova, todo.nova, lab.nova, unique.nova
- `bootstrap/` — Python interpreter: nova_lexer.py, nova_parser.py, nova_interpreter.py,
  nova_cli.py (v0.12: tested end-to-end, 191/191 green via tests/run_tests.py)
- `tests/` — run_tests.py end-to-end suite (subprocess-based)

## 2. Key Decisions

**Language core (decision log in README.md):**
- Memory: ARC default + escape analysis; `owned` opt-in; `unsafe` raw; GC only via `--runtime full`
- `dynamic` = complete runtime system (tagged values); specialization is a later optimization
- Interop: C ABI first + C header import; no JVM backend in v1 (JNI-style bridge)
- Runtime profiles: `minimal` (<100KB) / `core` / `full`; stdlib never calls runtime directly
- Concurrency: `parallel` = compiler-scheduled tasks + explicit `spawn`/`Channel`/`select`
- Strings UTF-8; ints fixed-width (i32 default, overflow panics debug / wraps release); BigInt stdlib
- Errors: `Result`/`Optional` + `?` primary; `throw`/`catch` = panic handling, never control flow

**Syntax (Nova Natural is PRIMARY):**
- English phrases: `say`, `ask "..." and remember it as x`, `set x to`, `add n to`,
  `if ... then ... otherwise ... done`, `repeat until/forever/N times/for each ... done`,
  `to greet with name ... done`, `when the program starts ... done`, `check x / when it is ...`
- Blocks terminated with `done` (no indentation sensitivity). Statements end at newline.
- Multi-line `then`/`otherwise` bodies REQUIRE `done`; single-line inline forms do not.
- Field access ALWAYS `the <field> of <obj>` (bare `field of obj` is not parsed).
- Compact symbol syntax (`{}`, `=>`, `print`) remains valid shorthand; both skins → identical AST.

**Approved feature packs (integrated into specs):**
- 16 extensions (docs/EXTENSIONS.md): refinement types, units, verified format-strings,
  pipelines/`then`, signals, actors, contracts, capability `[permissions]`, reproducible builds,
  time-travel debugger, notebook mode, `nova explain`, API-diff, edu-pack/blocks editor,
  embedding API, native hot reload
- 13 unique features (specs/unique_features.md): Flow<T> (one API for lists/streams/channels),
  Table primitive + `.ntab` + SQL pushdown, `track`/`undo`/variable-history queries,
  typed taint tracking, kernel state machines, `exact` math blocks, deterministic `--sim`,
  `@incremental`, time statements (`every day at 09:00`), `nova why`, grammar literals,
  **pure-Nova stdlib** (only OS syscalls; own regex/TLS/db/compression)

**Implementation strategy:**
- Python 3.13 bootstrap interpreter FIRST (machine has no g++/cl/cargo); native C++ LLVM
  pipeline follows later per docs/ROADMAP.md M0
- Bootstrap scope: Natural syntax subset sufficient for `guessing_game.nova` + `todo.nova`
  (not lab.nova / unique.nova — units, actors, signals, tables unsupported → clean errors)

## 3. Actionable Commands & Code Snippets

```powershell
# Run programs (once examples are fixed, see Next Steps)
python bootstrap/nova_cli.py run examples/guessing_game.nova
python bootstrap/nova_cli.py --seed 42 run examples/guessing_game.nova   # deterministic random
python bootstrap/nova_cli.py parse examples/todo.nova                    # AST dump (golden tests)
python bootstrap/nova_cli.py version
```

**Nova Natural sample (target the interpreter must run):**
```text
when the program starts
    secret is a random number between 1 and 100
    repeat until the guess is the secret
        answer is ask "Dit gæt: "
        if answer is not a number then
            say "Det er ikke et tal — prøv igen."
        otherwise
            set guess to the number value of answer
            if guess is less than secret then say "Højere!"
        done
    done
done
```

**Parser invariants (bootstrap/nova_parser.py):**
- Statements do NOT consume trailing NEWLINE — block loops (`p_body`, `p_block`,
  `parse_program`) own newline skipping. Inline bodies end because NEWLINE is next token.
- `p_body(stop_words)` returns `(Block, used_done)`; `used_done` drives required `done` in `p_if`.
- `the F of OBJ` chains built in `parse_the_chain()` (also: contents of / first|last item of /
  number value of / length of). Generic postfix `of` was REMOVED (reversal bug source).
- `parse_term_first_only()` = arithmetic WITHOUT `times` operator (for `repeat N times`,
  `between A and B`, `item N of`).
- **`the number value of X` binder X på factor-niveau (v0.12/C03):** ellers sluger
  NumVal hele aritmikken (`nv x? plus 1` blev `NumVal(x plus 1)`), `?`-giften når
  aldrig konverteringen, og `"5" * 2` inde i frasen gav `"55"`. Samme princip som
  `between A and B` / `item N of`. For numeriske X er resultatet uændret.
- **Stdlib (v0.12/B03):** `use [the] standard NAVN [library]` → `_stdlib_name()`
  validerer formen; binder NAVN (lavere case) til ModuleInstance fra
  `STDLIB_FACTORIES` (cache i `Interp._stdlib` — dobbelt-use = samme instans).
  `BuiltinFunction(name, params, fn)` kaldes via ModuleCall-vejen med arity-tjek;
  fejl = almindelige fangbare NovaErrors med linje. Navnet efter `.` er
  attribut-adgang og reserver-checks IKKE (file.write virker; B02's 11
  bindingssteder uændrede). C06/C07/C08 udvider STDLIB_FACTORIES + test-
  fragmentet for ukendt-lib-listen.
- **REPL (v0.13/C09):** `_run_repl()` i nova_cli.py — én persistent `Interp`;
  'done'-familie-parsefejl fortsætter bufferingen (`_is_open_block_error`), andre
  fejl rapporteres og rydder buffer. Linjer der IKKE kan parse som sætning men SOM
  ét udtryk (`_try_parse_expr_only`) echoes som `→ værdi` via `_repl_eval_echo`
  (snapshot til undo-stakken sker FØR eval). `:undo` gendanner dyb kopi af
  globals.vars + funcs + things (max 100; allerede printet tekst/fil-I/O rulles
  ikke tilbage — siges i :help). Echo kører udtrykket ÉN gang (ingen dobbelt-
  evaluering: ExprStmt-echo bruger eval direkte, ikke run()).
- **Moduler (v0.12/C05):** `p_usemodule()` kræver navn der ender på `-module`
  (entydigt vs. `[the] NAVN is ...`-deklarationer — `the save-file is "x"` rammer
  IKKE branchen, fordi ahead(2) er "is" ikke "in"). Postfix: DOT + WORD + LPAREN
  → `ModuleCall` KUN når basen er bar Var (ellers gammel "kommer senere"-fejl);
  ellers Field. Fortolker: `ModuleInstance(name,path,funcs,things,scope parent=None)`
  — fuld isolation; `_load_module` tjekker import-stakken FØR cache'en (ellers
  opdages cirkulær import aldrig — modulet er allerede cachet under sin egen
  kørsel); `_cur_dir` save/restore omkring modul-krop (relative imports kæder
  op imod den importerende fil). `_invoke(fn,args,line,parent)` = fælles vej for
  call() og ModuleCall — arity-tjek ligger HER (ModuleCall må ikke kunne springe
  det over). CLI: `root_dir=dirname(programfil)`; NovaLex/ParseError fra modul-
  indlæsning under run() fanges nu af CLI (ingen traceback-lækage).
- **RESERVED_WORDS (v0.11/B02):** frozenset i `nova_parser.py`; alle navne-bindingssteder
  går gennem `expect_name(what)` (WORD-tjek + reservations-fejl "— vælg et andet navn").
  Politik + ordliste: specs/syntax/lexical.md §Reserverede ord. `number/length/first/
  last/count` er bevidst FRI som navne.
- **`my x is E`** er gyldig deklaration (sugar, lig `set my x to`).

- **Golden AST-dumps (v0.11/B05, bootstrap/nova_dump.py):**
- Formatkontrakt står i nova_dump.py-filhovedet; native M0 skal matche byte-for-byte.
- tests/golden/*.nova + .ast.txt; kør `python tests/run_tests.py --update-goldens` efter
  BEVIDSTE format-/grammar-ændringer; determinisme dobbelt-kørsel testes automatisk.
- Goldener afslørede BOM-bug (lexer stripper nu UTF-8 BOM), `multiplied by`-crash og
  paren-crash — behold korpusset som regressionsnet ved alle grammar-ændringer.

**Kompakt skin (v0.11/C01):**
- Lexer: `_SKIN_SINGLE/_SKIN_DOUBLE`; to-tegns tokens matches FØR enkelte (`==` før `=`).
  Bindestreg-policy: `-` + bogstav → fortsæt ord (`save-file`); `-` + ciffer → MINUS
  (`x-1` = subtraktion); selvstændig `-` = altid MINUS. Kilde: specs/syntax/lexical.md.
- Parser-kæde: comparison(+==,!=,<,<=,>,>=) → arith(+,-) → term(*,/,%,over,times) →
  factor(unær ! og -; ord-"not" stadig løs) → **postfix(.felt)** → primary. Symbol-op
  `Bin`-strenge = IDENTISKE med ord-skin (plus/minus/times/divided/mod/gt/lt/gte/lte/
  eq/ne/and/or). Unær minus desugares til `0 - x` (ingen ny node).
- Statement: `navn.felt = expr` / `navn = expr` gren ligger FØR `[the|my] NAME is`-
  fallbacken og scanner lookahead (WORD (DOT WORD)* EQUALS) før commit.
- `{ }` lexes rene men har ingen statement-grammatik endnu (C10/T3); `.metode(` fejler
  pænt med "kommer i en senere version". Fortolkeren er UÆNDRET — skinnet lever kun i
  lexer/parser (kontrakt §4.5 holdt).
- Kryds-skin-ligevægt: 5 permanente par i `test_shorthand` sammenligner dumps
  byte-for-byte; udvid ved hver ny konstruktion (C02-politik).

**Interpreter notes (bootstrap/nova_interpreter.py):**
- Control flow via exceptions: `BreakSignal`, `ContinueSignal`, `ReturnSignal`, `ExitSignal`,
  `NothingSignal` (v0.12/C03).
- **Optional (v0.12/C03):** parseren pakker hvert udtryk der bar en `?` i ÉN
  `QuestionE` ved roden (`wrap_optional()` i nova_parser.py — markere findes ALDRIG
  indlejret i dumpede AST). Fortolkeren kaster `NothingSignal` på præcis to steder
  (Bin regne-/rækkefølge-operand = nothing; Field-læsning af nothing); QuestionE-
  eval er eneste catch-site → returnerer NOTHING. Ingen eksplicit dybdetæller:
  Python-unwinding er grænsemekanikken, og grænsen er leksikalsk/AST-rodfast.
  eq/ne med nothing er fritaget (det ER testen: `if x is nothing`). `try ... if it
  fails` fanger IKKE NothingSignal (fravær ≠ fejl). Uden guard konverterer CLI'en
  signalet til sætning + fix-hint. `plus` med blandet tal/tekst = NovaError-sætning
  (var rå TypeError).
- `Scope` chains to globals; assignment declares-in-place. `ThingInstance` = class + fields dict.
- `truth()` enforces bool-only conditions (language rule: no truthiness).
- Tracked vars (`track x`) snapshot via `copy.deepcopy` on mutation → `undo`/`redo`.
- `--seed N` → `random.seed()` for deterministic `a random number between A and B`.

## 4. Next Steps — FULDFØRT i v0.11

1. ✅ `examples/guessing_game.nova`: manglende `done` for if/otherwise-kæden tilføjet.
2. ✅ `examples/todo.nova` omskrevet til parser-kompatibel Natural (`store ... as json`,
   `load-tasks()`, `the <felt> of <obj>` overalt, dobbelt-`done` omkring nøstede
   if/otherwise-kæder inde i check-arms, EOF-guard-arm `when it is empty`).
3. ✅ `tests/run_tests.py` oprettet (end-to-end via subprocess): CLI-verber, ~30
   sprogsætninger, undo/redo, contracts, IO-fejl, begge eksempler inkl. persistens.
4. ✅ Tests kørt grønne: 49/49.
5. Later: C++ M0 per ROADMAP (lexer + Pratt parser + golden `dump` tests); then IR/LLVM.
6. ~~Open question: keep `requires`/`ensures` evaluated eagerly?~~ **Besluttet (v0.11):**
   `requires` eager ved kald; `ensures` udskydes til funktionsafslutning og evalueres i
   funktions-lokalt scope (post-betingelser ser sluttilstanden). Dokumenteret i README.

## 5. Changelog (v0.11 →)

**2026-08-23 — C05 moduler (v0.12, 159/159):**
- Parser: `UseModule` + `ModuleCall` noder; `p_usemodule()`; postfix DOT+LPAREN
  på bar Var → ModuleCall (andre baser beholder "kommer senere"-fejlen).
- Fortolker: `ModuleInstance`, `_load_module` (stak-først-cache, kæde-fejl,
  venlige fejl for manglende fil/parse-fejl med filnavn, `when the program
  starts` forbudt i moduler), Field-læsning af modul-vars/funktioner,
  `_invoke`-refaktor (arity delt). CLI: root_dir + run()-fejlcatch.
- Testafslørrete huller: cirkulær import usynlig pga. cache-før-stak rækkefølge;
  ModuleCall sprang arity-tjek over; manglende `done` i testens modul afslørede
  traceback-lækage for parse-fejl under kørsel → CLI-catch tilføjet.
- Beviser: 12 module-tests (import/kald/læsning, isolation, idempotens, kædet
  relativ import, 6 fejltilfælde), kryds-skin-par 7, golden 19.

**2026-08-23 — C03 Optional/`?` (v0.12, 146/146):**
- Lexer: `?` → QUESTION (skin-symboltabel). Parser: `QuestionE`-node + postfix i
  `parse_postfix()` + `wrap_optional()`/`_strip_question()` ved hvert `parse_expr()`-
  toppunkt (gælder også `{...}`-interpolation og parenteser — indlejrede pakninger
  komponerer til én rod-pakning). Dump: QuestionE er en almindelig node (dumper uændret).
- Fortolker: `NothingSignal(line,msg)`; throw-sites = eval_bin regne-/rækkefølge-guard
  + Field af nothing; catch-site = QuestionE-eval. CLI: NothingSignal → "Nova-fejl —
  linje L: ..." rc=1.
- Testafslørrete fixes: `the number value of X` → `parse_factor()`-operand
  (giften lå udenom NumVal; `"5" * 2` gav `"55"`); str+tal `plus` → sætning + hint.
- Beviser: 11 optional-tests (inkl. poison-case, logic-error-not-covered,
  marker-position-free, double-marker), kryds-skin-par 6 (`u = q? + 1` ≡
  `set u to q plus 1?`, byte-identiske dumps), golden `18-optional`; alle 17 gamle
  goldener uændrede.

**Parser-fixes (bootstrap/nova_parser.py):**
- `parse_the_chain()`: indbyggede fraser (`contents/first item/last item/number value/
  length`) tjekkede forkert lookahead-token (`peek(1)` i stedet for hoved-ordet) — alle
  fraserne var død kode; `the length of xs` mis-parsed som feltadgang og crashede runtime.
- `a random number between A and B`: tredobbelt `next()` åd "number" og krævede endnu en
  — headline-featuren kunne slet ikke parses. Nu: to `next()` + `expect number/between`;
  accepterer også "an".
- `every item of X turned into a T`: åd ikke artiklen ("a/an") før thing-navnet.
- `check`/`when it is ...`: mønster-parseren konsumerede aldrig "is" → alle
  ligheds-mønstre fejlede.

**Fortolker-fixes (bootstrap/nova_interpreter.py):**
- `_pat_match()` evaluerede mønster-værdien *før* `isempty`-tjekket (val=None) →
  `when it is empty` crashede med "ukendt udtryk NoneType".
- `track x` + første tildeling crashede ("variablen findes ikke", linje 0):
  `_snapshot()` tager nu kun snapshot hvis variablen findes.
- Contracts: `ensures` udskydes via `_ensure_frames` til funktionens afslutning;
  `requires` forbliver eager.
- Fil-IO (`contents of`, `store ... in`) pakkes ind i `NovaError` (missing file, mappe,
  ugyldig UTF-8, ugyldig json) → fangbar med `try ... if it fails`.
- `ask`/input ved ægte stdin-EOF raise `ExitSignal` (pænt stop) i stedet for uendelig
  løkke på tomme strenge.
- Imports flyttet fra løkke-kroppe til modultop; `wait` understøtter hours.

**Lexer/CLI:**
- Lexer: kolonne-sporing (Token.col) + kolonne i lex/parser-fejlmeddelelser
  ("linje L, kolonne C") — første skridt mod ROADMAPs span-krav.
- `nova version` kræver ikke længere et fil-argument; `--seed` valideres; VERSION =
  0.11.0-bootstrap.

**Eksempler:** guessing_game.nova (balanceret done-struktur) og todo.nova (fuldt
omskrevet) kører begge end-to-end inkl. JSON-persistens på tværs af kørsler.

**Kendte huller i v0.12 (opdateret 2026-08-23 — gamle huller lukket):** kompakt
shorthand-skin ✓ (C01), unary minus ✓ (B04), Optional/`?` ✓ (C03), moduler ✓ (C05),
stdlib v0 ✓ (B03+C06+C07+C08). TILBAGE: Result-typede fejl (C04), lambdas/pipelines
(C10/T2), check-exhaustiveness-lint (C11), REPL (C09 — i gang), formatter/linter
(D02/D03), tour/lab/unique.nova er STADIG udenfor bootstrap-scope (fejl pænt med
sætninger — verificeret 2026-08-23); spans kun i frontend-fejl (ikke AST);
`.metode(` på ikke-modul-værdier fejler bevidst pænt indtil C10; indbyggede
fraser (`the contents/first/last/length/number value of ...`) vinder over
samme-namnede felt-navne i `the ... of ...`-form — brug prik-adgang (`x.length`).

## 6. Original Next Steps (historisk)

1. **Fix `examples/guessing_game.nova`**: add `done` closing the multi-line
   `if answer is not a number then ... otherwise ...` chain (before the repeat's `done`).
2. **Rewrite `examples/todo.nova`** to parser-compatible Natural:
   - `write tasks to the save-file as json` → `store tasks in the save-file as json`
   - `tasks is the tasks loaded from the save-file` → `tasks is load-tasks()`
   - all field access → `the text of task`, `the finished of item number of tasks`,
     `set the finished of item number of tasks to true` (lvalue chain supported)
   - add `done` to every multi-line if/otherwise chain inside `check` arms
3. **Create `tests/run_tests.py`** (end-to-end, subprocess):
   - guessing game: stdin = `["abc"] + list(range(1,101))`, `--seed 7`; assert output contains
     "Jeg tænker", "Det er ikke et tal", "Rigtigt!", "Du brugte"
   - todo: run in temp cwd; stdin `["tilføj køb brød","vis","færdig 1","vis","farvel"]`;
     assert `[ ] 1)` then `[X] 1)` and todo.json exists; second run stdin `["vis","farvel"]`
     must show `[X] 1) køb brød` (persistence proof)
   - `parse examples/todo.nova` exits 0; a bad program exits 1 with "linje" in stderr
4. **Run tests, debug** parser/interpreter until green (`python tests/run_tests.py`).
5. Later: C++ M0 per ROADMAP (lexer + Pratt parser + golden `dump` tests); then IR/LLVM.
6. Open question: keep `requires`/`ensures` evaluated eagerly (current) or only in debug profile.
