# Nova — Iteration Plan (v0.11 → 1.0)

> ## ⚠ MAINTENANCE RULE — READ FIRST
> **This file is the single source of truth for what to do next and what is done.**
> Every time ANY item is completed (or a new item is discovered), update — in the same
> session, before committing:
> 1. the **status column** in §6 (Phase backlog) and §7 (Priority list),
> 2. the **Changelog** in §11 with date + one line,
> 3. `project-notes.md` §changelog if implementation details matter.
>
> An item is only "Done" when its Definition of Done (§5) is fully met.

---

## 1. Mission & product principles

**Elevator pitch:** *Nova is the language where you describe the app and it gets built.*
You write what you mean in plain English sentences (`Nova Natural`), run it, and it
works. When you grow, the language grows with you — same semantics, denser syntax,
deeper control.

**Design law — the three-tier skill curve:**

| Tier | Who | Syntax | Power |
|---|---|---|---|
| T1 Natural | Day 1 beginner | English sentences, `done`-blocks | Full language: loops, functions, things, files, JSON, contracts |
| T2 Shorthand | Intermediate | `{}`, `=>`, symbols, `x = 10` | Same AST as T1, plus collections/pipelines/lambdas ergonomics |
| T3 Systems | Expert | annotations, generics, traits, `unsafe`, ARC/ownership control | C++-class performance, FFI, metaprogramming |

Rules that make "describe it and it's built" true:
1. **No dead ends.** Everything expressible in shorthand MUST be expressible in
   Natural (and vice versa). Enforced by cross-skin equivalence tests.
2. **One obvious way** at T1 per concept; extra ways are additive, never required.
3. **Errors are sentences**, not codes: every diagnostic says what happened, why, and
   the exact words to fix it (`nova explain <code>` later).
4. **Zero config**: `nova run app.nova` just works; stdlib ships batteries-included.
5. **Simple default, deep ceiling**: ARC + dynamic by default; types/perf opt-in.

## 2. What "on par with commercial languages" means (parity matrix)

Nova 1.0 is "on par" when every row reaches the level of Python (usability) +
Java (ecosystem/tooling) + C++ (performance). This matrix is audited each phase gate.

| Area | Parity bar (measurable) | Status v0.11 |
|---|---|---|
| Syntax & semantics | Two skins → identical AST; full spec implemented; exhaustive match | 🟡 bootstrap subset only |
| Type system | HM-local inference, unions, Option/Result, generics+traits | 🔴 dynamic-only in bootstrap |
| Memory model | ARC default, escape analysis, `owned`/`unsafe` opt-in, cycle collector (full) | 🔴 GC'd by host (Python) |
| Error handling | Result/Optional + `?`; catchable runtime errors w/ codes+hints | 🟡 NovaError + try/catch |
| Concurrency | async/await, structured concurrency, channels/select, `parallel` | 🔴 |
| Stdlib | io/fs/net/http/json/csv/time/math/random/test/cli ≥ Python-parity table (specs/standard_library.md) | 🔴 ~5 builtins |
| Tooling | REPL, formatter, linter, LSP, debugger(DAP), test runner, doc generator | 🔴 CLI only |
| Package ecosystem | semver registry, lockfile, workspaces, `project.nova` | 🔴 |
| Performance | fib/matmul/sort within 2× of C++ (LLVM), 100× of CPython on hot loops | 🔴 interpreter |
| Distribution | Single-file static executables; cross-compile matrix | 🔴 needs Python |
| Docs & learnability | Language reference complete, tutorial, playground, error index | 🟡 specs done, tutorial missing |
| Unique differentiators | Flow, Table, track/undo, taint, state machines, time statements… | 🟡 track/undo only |

## 3. Current status dashboard (v0.12-bootstrap — G0 CLOSED 2026-08-23)

- ✅ G0 closed: suite green (191 tests), both examples perfect, error audit done (B01)
- ✅ Python bootstrap lexer/parser/tree-interpreter, Natural + shorthand skins
- ✅ Contracts (`requires` eager / `ensures` deferred), track/undo/redo
- ✅ Optional/`?` whole-expression poisoning; real modules with namespaces; stdlib v0
  (json/file/random/time/math/text/list via `use the standard X library`)
- ✅ End-to-end suite: 191 tests green (`python tests/run_tests.py`); golden dumps 01–19
- ❌ Result-typed errors, REPL, lambdas/pipelines, formatter/linter, native compiler — see backlog

## 4. Development workflow (do this every iteration, in order)

**The loop (one item at a time):**

1. **Baseline:** run `python tests/run_tests.py` — must be green before starting.
2. **Pick one item** from §7 Priority list (highest P, then lowest ID) that has no
   unfinished dependency.
3. **Spec first (language-visible behavior):** edit the relevant `specs/*.md` BEFORE
   coding; write the decision rationale in one sentence. If it changes a core decision,
   add a row to README's decision log.
4. **Tests first:** add failing tests to `tests/run_tests.py` (behavior + error cases)
   or a golden file under `tests/golden/`. Error-message quality is part of the test.
5. **Implement the smallest correct version.** No drive-by refactors, no scope creep:
   new ideas go to §9 Backlog parking lot, not into the current item.
6. **Verify:** full suite green + all examples still run + manual smoke of the affected
   example. Never leave the repo red.
7. **Update docs (mandatory):**
   - `docs/ITERATION_PLAN.md`: status → ✅ Done (date), §11 changelog line;
   - `project-notes.md`: parser/interpreter invariants if they changed;
   - `specs/*.md`: mark feature as implemented where relevant;
   - `README.md`: only for decision-level changes or new user-facing verbs.
8. **Commit** (one logical item per commit, message = item ID + short summary).
9. Repeat from 1.

**Definition of Done (per item):**
- [ ] Tests written before/with code; suite green
- [ ] Spec + plan + notes updated
- [ ] Example updated/added if user-visible
- [ ] Error messages follow rule 3 (sentence + fix hint)
- [ ] No known regression; performance not measurably worse

**Version scheme:** `0.MINOR.PATCH`. One MINOR bump per completed phase-step cluster;
PATCH for fixes. 1.0 only when §8 freeze criteria hold.

### 4.5 Excellence contracts (per risky item — written BEFORE implementation)

Risky items get an explicit contract here before code is touched. A contract names the
invariants that must survive, the traps, and the evidence required at review.

**C03 — Optional/`?` semantics:**

- **Invariant 1 — one rule, sentence-shaped:** *if any part of an expression is
  `nothing`, and the expression carries a `?`, the WHOLE expression is `nothing`.*
  Not per-operand, not per-call — the entire enclosing expression (documented in
  specs/error_handling.md before coding).
- **Invariant 2 — without `?`, nothing still fails loudly** with the existing friendly
  error. `?` never silences logic errors: out-of-bounds, unknown fields on real things,
  unknown functions keep raising; ONLY absence-of-value propagates.
- **Trap — marker vs boundary.** Naïve postfix wrapping (`x? + 1` guarding only `x`)
  is WRONG. Parse-level transform: if the finished tree contains any `?`, strip the
  markers and wrap the ENTIRE tree once. Equivalence golden must pin this.
- **Scope fence:** `?` token + QuestionE + NothingSignal at exactly two throw-sites
  (Bin operand-guard, Field read of nothing). No Result type, no `try?`, no method
  chains (later items).
- **Evidence required:** failing tests first incl. poisoning case that would crash
  unguarded; ≥1 cross-skin pair (`u = q? + 1` ≡ natural form); golden 18-optional;
  suite green; interpreter diff limited to NothingSignal sites + depth counter.

**C01 — compact-shorthand skin:**

- **Invariant 1 — ONE AST.** Shorthand must reuse the EXISTING node types and operator
  strings (`plus/minus/times/divided/mod/gt/lt/gte/lte/eq/ne/and/or`, `Field`,
  `Assign`). If a single interpreter edit seems necessary → STOP; that is a design
  smell; justify in writing or redesign the desugaring.
- **Invariant 2 — Natural is untouched.** Every existing golden dump must stay
  byte-identical. Word operators (`is greater than`) keep exact precedence behavior.
- **Trap: hyphen vs minus.** Identifiers contain `-` (`save-file`). Policy (spec'd in
  lexical.md BEFORE coding): inside a word, `-` followed by a letter continues the word;
  `-` followed by a digit ends it and lexes MINUS (so `x-1` = subtraction). Standalone
  `-` is always MINUS.
- **Trap: precedence drift.** Symbol ops slot INTO the existing level chain
  (`|| < && < == != < < <= > >= < + - < * / % < unary ! - < .field`); no new levels,
  no reordering of word levels.
- **Scope fence.** C01 = lexer symbols + `name = expr` assignment + symbol operators +
  dotted field read/write + unary `!`/`-`. NOT in scope: lambdas/`=>`, method calls
  `.map(...)`, `fn`/brace-bodies, types (→ C07/C10/T3 items). `{ }` lexes cleanly but
  has no statement grammar yet — say so in the spec to avoid "half-done" ambiguity.
- **Evidence required:** failing tests written first (incl. ≥3 cross-skin pairs whose
  AST dumps are byte-identical); suite green; new golden `17-shorthand.nova`; zero
  diffs on all pre-existing goldens; `git diff bootstrap/nova_interpreter.py` empty.

## 5. Phase gates

A phase is DONE only when every exit criterion holds. Gates are cumulative.

- **G0 — Bootstrap trustworthy** *(✅ CLOSED 2026-08-23 at v0.12)*: suite green; both
  examples perfect; error messages audited once end-to-end.
- **G1 — T1 language complete** (v0.13–v0.19): 100% of natural_syntax.md parseable &
  runnable; Result/Optional; real module system; stdlib-core v0; REPL; golden AST dumps
  for every construct; cross-skin equivalence harness (shorthand parses → identical AST).
- **G2 — Language credible** (v0.20–v0.29): `nova test` runner + assert lib; formatter
  v0 (idempotent); linter v0 (naming/safety); deterministic `--sim` formalized; docs
  site skeleton + tutorial rewritten around describe-the-app; **freeze unique-feature
  design (no implementation yet)**.
- **G3 — Native pipeline alive** (v0.30–v0.49): C++ toolchain installed & CI'd; M0
  native lexer/parser byte-compatible with Python golden dumps; Nova IR + verifier;
  LLVM backend runs guessing_game/todo natively; ARC pass; differential testing
  native-vs-bootstrap on the corpus.
- **G4 — Parity tooling** (v0.50–v0.69): VM bytecode + JIT-lite; LSP v1 (hover, goto,
  diagnostics); DAP debugger; package manager + lockfile (+registry or vendoring);
  stdlib to parity-table v1; async/await + channels.
- **G5 — Differentiators shipped** (v0.70–v0.89): Flow<T>, Table(+SQL pushdown),
  history engine (undo/debug/revision unified), state machines, time statements,
  `nova why`/`explain`, capability sandbox.
- **G6 — 1.0-ready** (v0.90+): freeze criteria §8 met; edition-2027 spec freeze;
  benchmark suite published; 10-app gallery incl. one self-hosted tool.

## 6. Phase backlog (live status — keep updated)

Statuses: ☐ Not started · ◐ In progress · ✅ Done (date) · ⏸ Blocked (by ID)

### Phase 0 — Bootstrap hardening (→ G0)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| B01 | Error-message audit: every NovaError = sentence + fix hint; catalog in tests | P0 | M | — | ✅ 2026-08-22 |
| B02 | Reserved-word policy: document builtin phrases that can't be variable names; enforce with clear error | P0 | S | — | ✅ 2026-08-22 |
| B03 | `use standard X` maps to real stub modules (random/json/file/time/math) instead of being ignored | P1 | M | — | ✅ 2026-08-23 (BuiltinFunction-moduler via ModuleCall-maskineriet; use-form valideret; ukendt lib = venlig fejl) |
| B04 | Unary minus + negative literals in expressions | P1 | S | — | ✅ 2026-08-22 (via C01: `0 - x`-desugaring, begge skins) |
| B05 | Golden AST dump tests: one file per construct under tests/golden/, compared via `parse` | P0 | M | — | ✅ 2026-08-22 |

### Phase 1 — T1 complete (→ G1)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| C01 | Compact-shorthand skin: lexer symbols `= { } . , ( ) => > < + - * /`, expression grammar; SAME AST nodes | P0 | L | B05 | ✅ 2026-08-22 (uden lambdas/`=>`, `.metode()`, `fn{}` — se kontrakt-fence) |
| C02 | Cross-skin equivalence golden tests (Natural vs shorthand pairs) | P0 | M | C01 | ✅ 2026-08-22 (5 permanente byte-identiske par i test_shorthand; udvides ved hver ny konstruktion) |
| C03 | `Optional<T>` values: `nothing` checks, `?` postfix propagation in call chains | P0 | M | — | ✅ 2026-08-23 (hele-udtryksgift: markere strippes, træ pakkes ét sted i QuestionE; 2 throw-sites; golden 18 + kryds-skin-par 6) |
| C04 | `Result` pattern: `try ... if it fails as err ... done` returns typed result; `give back ok/err` | P1 | M | C03 | ☐ |
| C05 | Modules: `the tools-module in "tools.nova"` import; namespaces; circular-import error | P0 | M | — | ✅ 2026-08-23 (bootstrap-udsnit i module_system.md §0: import + navnerum + parentes-kald + cirkulær/venlige fejl; golden 19; par7) |
| C06 | String library v0: upper/lower/trim/split/join/replace/length-of/contains/at/slice | P0 | M | — | ✅ 2026-08-23 (text.* builtins; 1-baseret at/slice; alle-fejl = sætning+hint) |
| C07 | List/dict library v0: sort/reverse/map(→T2)/filter/fold/min/max/keys/values | P1 | M | C06 | ✅ 2026-08-23 (sort/reverse/min/max/keys/values; map/filter/fold udskudt til C10/T2 som aftalt) |
| C08 | Math/time/random library v0 per standard_library.md subset | P1 | S | B03 | ✅ 2026-08-23 (math: abs/floor/ceil/pow/PI · time: sleep · random: shuffle kopi+seedbar) |
| C09 | REPL: `nova repl` — persistent Interp session, `:ast/:undo/:quit`, multiline via `done` | P1 | M | — | ✅ 2026-08-23 (+ `:help`, udtryks-echo `→ værdi`, fejl dræber ikke sessionen, seedbar determinisme) |
| C10 | Lambdas + pipeline `then` (T2 ergonomics over lists) | P2 | M | C07 | ☐ |
| C11 | match-exhaustiveness lite: `check` warns on missing otherwise (lint) | P2 | S | — | ☐ |
| C12 | Bootstrap perf pass: memoize dispatch tables; target 2× on todo bench | P2 | M | — | ☐ |

### Phase 2 — Credible language (→ G2)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| D01 | `nova test`: discovers `*.test.nova`, asserts lib, reports diffs | P1 | M | C05 | ☐ |
| D02 | Formatter v0: token-stream based, idempotency property test | P1 | L | C01 | ☐ |
| D03 | Linter v0: naming, unused var, missing otherwise, truthiness trap | P2 | M | D01 | ☐ |
| D04 | `--sim` determinism mode: seed everything, forbid wall-clock/random-without-seed | P2 | M | — | ☐ |
| D05 | Tutorial "Describe your first app" + docs site skeleton (md → static html) | P1 | M | — | ☐ |
| D06 | Unique-feature design freeze doc (Flow/Table/history/taint/state-machines APIs) | P1 | M | — | ☐ |

### Phase 3 — Native pipeline (→ G3)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| E00 | Install MSVC Build Tools + CMake + LLVM (winget) or accept Rust alternative decision; record in README log | P0 | S | — | ☐ |
| E01 | GitHub Actions CI: Python suite on windows+ubuntu (fast feedback forever) | P0 | S | — | ☐ |
| E02 | Native lexer+parser (C++), golden-dump byte-compatible with Python output | P1 | L | E00,B05 | ☐ |
| E03 | Nova IR (SSA) + verifier + text form `nova ir dump` | P1 | L | E02 | ☐ |
| E04 | LLVM backend: integers/strings/lists enough to run guessing_game natively | P1 | XL | E03 | ☐ |
| E05 | ARC insertion pass + escape analysis | P1 | L | E04 | ☐ |
| E06 | Differential tester: run corpus through bootstrap & native, diff outputs | P0 | M | E04 | ☐ |
| E07 | VM bytecode backend + stack interpreter (REPL/scripting tier) | P2 | XL | E03 | ☐ |

### Phase 4 — Parity tooling (→ G4)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| F01 | LSP v1: hover, goto-def, find-refs, diagnostics, inlay hints | P1 | XL | E02 | ☐ |
| F02 | Debugger: DAP server on VM breakpoints/step/vars | P2 | XL | E07 | ☐ |
| F03 | Package manager: `project.nova`, resolve/lockfile, cache; registry can start as git-tags | P1 | L | C05 | ☐ |
| F04 | async/await + scheduler + Channel/select in native runtime | P1 | XL | E05 | ☐ |
| F05 | stdlib to parity-table v1: fs/path/env/process/net/http/cli/log | P1 | XL | F03 | ☐ |
| F06 | Doc generator from `///` comments; error-code index (`nova explain`) | P2 | M | D05 | ☐ |

### Phase 5 — Differentiators (→ G5)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| H01 | History engine unification: track/undo feeds time-travel debug + revision queries (`ever`, `when was`) | P2 | L | F02 | ☐ |
| H02 | Flow<T>: one API lists/streams/channels | P2 | L | F04 | ☐ |
| H03 | Table primitive + CSV/JSONL + SQL-pushdown-lite | P2 | L | F05 | ☐ |
| H04 | Kernel state machines (`a workflow is ...`) | P2 | M | — | ☐ |
| H05 | Time statements `every day at 09:00` (scheduler in runtime) | P2 | M | F04 | ☐ |
| H06 | `nova why` (explain last error path) + typed taint tracking | P3 | L | F01 | ☐ |
| H07 | Capability permissions `[permissions]` + sandboxed run | P3 | L | F02 | ☐ |

## 7. Priority list (flat, always sorted — work top-down skipping blocked)

1. **B05** golden dumps — foundation for everything native (P0)
2. **B01** error audit (P0)
3. **B02** reserved words (P0)
4. **C01+C02** shorthand skin + equivalence — the two-skins promise kept or dropped (P0)
5. **C03** Optional/nothing semantics (P0)
6. **C05** modules (P0)
7. **C06–C08** stdlib v0 trio (P0/P1)
8. **B03/B04** quick wins (P1)
9. **C09** REPL (P1)
10. **E00+E01** toolchain install + CI (unblocks all of Phase 3 early!) (P0)
11. **D01/D05/D06** credibility pack (P1)
12. **C04/C10/C11/C12** (P1/P2)
13. **D02/D03/D04** (P1/P2)
14. **E02→E06** native pipeline sequence (P1)
15. **F-series** (P1/P2)
16. **H-series** (P2/P3)

> Rule: never start an H-item while any P0/P1 above it is unfinished.

## 8. 1.0 freeze criteria (all must hold)

1. Parity matrix §2: every row ≥ its bar except GPU/mobile (explicitly post-1.0).
2. Native LLVM build runs all gallery apps; ≤2× C++ on fib/matmul/sort benchmarks.
3. Differential fuzzing native↔bootstrap clean for 10k program corpus.
4. LSP hover/goto/diagnostics + idempotent formatter + working debugger demo video.
5. Package manager with versioned deps used by ≥3 third-party sample projects.
6. Tutorial, language reference, error index complete; playground online.
7. Spec frozen ("Edition 2027"); semver policy adopted; deprecation process written.
8. Zero known P0/P1 bugs open.

## 9. Crucial applications — the dogfood ladder

Each app proves a capability slice and becomes a permanent test/gallery piece.
"Gate" = earliest phase where building it must be possible.

| # | App | Proves | Gate | Status |
|---|---|---|---|---|
| A1 | Guessing game | teachability, I/O, loops | G0 | ✅ v0.11 |
| A2 | Todo (JSON persist) | CRUD, things, contracts, files | G0 | ✅ v0.11 |
| A3 | Notes CLI (args, dates, markdown files) | argv parsing, fs, time lib | G1 | ☐ |
| A4 | CSV → stats report (table printer) | strings, math, formatting | G1 | ☐ |
| A5 | Mini web server (JSON API, routes) | net/http, async — concurrency story | G4 | ☐ |
| A6 | Chat client+server | channels/select, structured concurrency | G4 | ☐ |
| A7 | Text adventure | kernel state machines (H04) showcase | G5 | ☐ |
| A8 | Data dashboard from CSV w/ undoable edits | Table + history engine (H01/H03) | G5 | ☐ |
| A9 | `nova fmt` written IN Nova | self-hosting seed, reflection/parser API | G6 | ☐ |
| A10 | GUI notes app | GUI toolkit post-1.0 direction | post-1.0 | ☐ |

**Showcase rule:** every phase gate ends with a 60-second screen-recording of the newest
A-app running. If it can't be recorded, the gate isn't done.

## 10. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Scope creep (13 unique features tempting while core is young) | Hard rule §7: H-items locked until their gate; ideas → parking lot below |
| Two-skin divergence | C02 equivalence goldens run in CI forever |
| No C++ toolchain on dev machine | E00 is a P0 prerequisite task, scheduled early, not "later" |
| Solo-dev bus factor / motivation | Small items (S/M), weekly visible win, showcase recordings |
| Windows-only paths/encoding bugs | E01 CI on ubuntu from day one; UTF-8 enforced everywhere |
| Bootstrap/native semantic drift | E06 differential tester part of CI at G3+ |
| Perf promises scare users away early | Bootstrap labeled reference-semantics oracle; perf marketing waits for G3 benchmarks |

**Parking lot (ideas, NOT commitments):** notebook/literate mode, blocks-editor for kids,
units/refinement types, actors/signals, GPU backend, grammar literals, `@incremental`.

## 11. Changelog (newest first — mandatory updates)

| Date | Change |
|---|---|
| 2026-08-23 | **C09 ✅**: `nova repl [--seed N]` — én persistent Interp; `>>> `/`..>`-prompter; multiline ved 'done'-familie-parsefejl (buffer fortsætter); udtryks-linjer echoes som `→ værdi` (fallback når sætnings-parse fejler — fix for tal-startede linjer); meta: `:ast <linje>` (udtryk først, fald tilbage til sætninger), `:undo` (dyb kopi af globals/funcs/things, stak max 100, output/fil-I/O kan ikke rulles tilbage), `:quit/:q`, `:help`, ukendt → venlig henvisning. Fejl dræber ALDRIG sessionen. Spec: docs/ARCHITECTURE.md §10. 8 repl-tests. Suite: 199/199. Næste: E00+E01 toolchain+CI (P0!) → D01/D05/D06. |
| 2026-08-23 | **docs-audit**: forældede påstande rettet (README shorthand-kommentar "planlagt" → implementeret; project-notes kendte-huller omskrevet til v0.12-virkelighed inkl. phrase-vs-feltnavn-kollision og prik-adgang-workaround; AGENTS.md 191/191). lab/unique/tour.nova verificeret at fejle med pæne sætninger (rc=1, ingen tracebacks). |
| 2026-08-23 | **v0.12.0-bootstrap + G0 LUKKET**: versionsbump (CLI + cli/version-test), §3-dashboard opdateret, README-statuslinje fikset (var forældet: 49 tests / "shorthand ikke implementeret"). G0-kriterier: suite grøn (191), begge eksempler perfekte, fejl-audit B01 ✅. Næste gate: G1 (T1 komplet) — C04/C09/C10/C11/C12 + D01/D05/D06. |
| 2026-08-23 | **C08 ✅**: math udvidet med abs/floor/ceil/pow + PI-konst (læses som modul-felt), time.sleep (afviser negative), random.shuffle (kopi, seedbar via --seed — determinisme-test tilføjet). 7 nye stdlib-tests. Stdlib v0-trioen (B03+C06+C07+C08) er dermed HEL. Suite: 191/191. Næste: C09 REPL → E00+E01 toolchain+CI (P0!). |
| 2026-08-23 | **C07 ✅**: list-biblioteket (sort/reverse/min/max/keys/values) som BuiltinFunctions; sort returnerer NY liste og afviser blandede typer med sætning; reverse er kopi; keys/values kræver databog (json.parse) og returnerer nøgle-sorteret. map/filter/fold bevidst udskudt til C10/T2 (kræver lambdas). 7 nye stdlib-tests (4 ok + 3 fejl). Suite: 184/184. Næste: C08 math/time/random fyldes op → C09 REPL. |
| 2026-08-23 | **C06 ✅**: text-biblioteket (upper/lower/trim/split/join/replace/length/contains/at/slice) som BuiltinFunctions; at/slice er 1-baserede (slice inklusiv); replace erstatter alle; join bruger nova_str pr. element; alle type-/grænse-fejl = sætning + gyldigt-interval-hint. 7 nye stdlib-tests (4 ok + 3 fejl). Suite: 177/177. Næste: C07 list → C08 math/time/random. |
| 2026-08-23 | **B03 ✅**: `use [the] standard NAVN [library]` binder NAVN til et BuiltinFunction-modul (samme ModuleCall-vej som C05; idempotent via Interp._stdlib). Biblioteker v0: json{parse,stringify}, file{read,exists,write}, random{between,pick}, time{now}, math{sqrt,round}. Ukendt lib/form = sætning med de tilgængelige. Test-fundet: `write` var reserveret som feltnavn efter `.` — dotted adgang er attribut-adgang, ikke binding, så reservations-tjekket der er fjernet (B02-bindingssteder uændret). ContentsOf genbruger nu _read_text_file-hjælperen. 11 stdlib-tests. Suite: 170/170. Næste: C06 text-biblioteket → C07 list → C08 math/time/random fyldes op. |
| 2026-08-23 | **C05 ✅**: `the X-module in "fil.nova"` (navn SKAL ende på `-module`; sti relativt til importerende fil), `ModuleInstance` med fuldt adskilte navnerum (funcs/things/scope parent=None), `modul.funktion(...)`-kald + felt-læsning af modul-vars, idempotent import-cache, cirkulær-import-fejl med hele kæden, moduler må ikke have `when the program starts`. CLI: root_dir = programmets mappe; fejl i modul-filer under kørsel fanges nu (ingen traceback-lækage). `_invoke`-refaktor: arity-tjek delt mellem `call()` og `ModuleCall` (test-fundet hul: modulkald sprang arity over). 12 module-tests + par7 + golden 19. Suite: 159/159. Næste: C06–C08 stdlib v0. |
| 2026-08-23 | **C03 ✅** (per kontrakt §4.5): `?`-token (QUESTION), postfix-markere i begge skins; `wrap_optional()` stripper markere og pakker HELE træet ét sted (`QuestionE` ved roden — golden 18 pin'er formen). Fortolker: `NothingSignal` + præcis to throw-sites (Bin regne-/rækkefølge-guard, Field-læsning af nothing); QuestionE-grænsen = eneste catch-site (Python-unwinding erstatter eksplicit dybdetæller — grænsen er leksikalsk/AST-rodfast). eq/ne med nothing ALDRIG gift (testen `if x is nothing` virker som før). Uden `?`: venlig sætning + fix-hint via CLI. To testafslørte fixes: (1) `the number value of X` binder X på factor-niveau — ellers lå giften udenom konverteringen og spec-eksemplet var umuligt (precedens: `between A and B`); (2) str+tal i `plus` gav rå TypeError → nu sætning + hint (B01-katalog case error/type-mismatch). 11 optional-tests + par6 + golden 18; alle 17 gamle goldener byte-identiske. Suite: 146/146. Næste: C05 (moduler) → C06–C08 (stdlib v0). |
| 2026-08-22 | **C01 ✅ + C02 ✅ + B04 ✅** (per excellence-kontrakt §4.5): lexer-symboler (`= + - * / % < > ! == != <= >= && || . { }`) med bindestreg-policy; `navn.felt = værdi` og symbol-operatorer afbildet på EKSISTERENDE op-strenge; unær `!`/`-` (`-x` → `0 - x`, ingen ny node); `.metode(` giver pæn "kommer senere"-fejl. **Nul fortolker-ændringer** (kontrakt holdt — verificeret). 16 shorthand-tests + 5 kryds-skin-par med byte-identiske dumps + golden 17. Suite: 132/132. Næste: C03 (Optional) → C05 (moduler). |
| 2026-08-22 | **B05 ✅** kanonisk AST-dumpformat (`bootstrap/nova_dump.py`, kontrakt i filhovedet) + 16 golden-par i `tests/golden/` + determinisme-tjek + `--update-goldens`. Goldenerne afslørede og fikstrakte 4 latente bugs: UTF-8-BOM ikke tolereret (spec-krav), `multiplied by` → UnboundLocalError, `(...)` → crash på manglende `postfix()`, `at least/most` kun gyldigt efter `is` (golden-kilde rettet). **B01 ✅** alle fejl = sætning + fix-hint; did-you-mean for variabler/felter/funktioner; exit-kode-katalog (19 cases) + "ingen Traceback-lækage"-assertions. **B02 ✅** RESERVED_WORDS (55 ord) i `nova_parser.py` + `expect_name()` ved alle 11 bindingssteder; politik dokumenteret i `specs/syntax/lexical.md`. Suite: 110/110. Næste: C01+C02 (shorthand-skin) → C03 (Optional). |
| 2026-08-22 | Plan created at v0.11-bootstrap; baseline: 49/49 tests, A1+A2 apps done, G0 in progress. |
