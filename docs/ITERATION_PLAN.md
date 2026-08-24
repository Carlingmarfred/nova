# Nova — Iteration Plan (v0.15 → 1.0)

> ## ⚠ MAINTENANCE RULE — READ FIRST
> **This file is the single source of truth for what to do next and what is done.**
> Every time ANY item is completed (or a new item is discovered), update — in the same
> session, before committing:
> 1. the **status column** in §6 (Phase backlog) and §7 (Priority list),
> 2. the **Changelog** in §11 with date + one line,
> 3. `project-notes.md` §changelog if implementation details matter.
>
> An item is only "Done" when its Definition of Done (§4) is fully met.

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
   the exact words to fix it (`nova explain <code>` later). All diagnostics are English.
4. **Zero config**: `nova run app.nova` just works; stdlib ships batteries-included.
5. **Simple default, deep ceiling**: ARC + dynamic by default; types/perf opt-in.

## 2. What "on par with commercial languages" means (parity matrix)

Nova 1.0 is "on par" when every row reaches the level of Python (usability) +
Java (ecosystem/tooling) + C++ (performance). This matrix is audited each phase gate.

| Area | Parity bar (measurable) | Status v0.15 |
|---|---|---|
| Syntax & semantics | Two skins → identical AST; full spec implemented; exhaustive match | 🟡 both skins + equivalence harness done (C01/C02); full spec missing |
| Type system | HM-local inference, unions, Option/Result, generics+traits | 🔴 dynamic-only in bootstrap (Optional/`?` done C03, Result → C04) |
| Memory model | ARC default, escape analysis, `owned`/`unsafe` opt-in, cycle collector (full) | 🟡 value/reference semantics pinned + `a copy of X` (C13); ARC/escape-analysis is native E05 |
| Error handling | Result/Optional + `?`; catchable runtime errors w/ codes+hints | 🟡 Optional/`?` done, English sentence errors + hints, catchable; typed Result missing |
| Concurrency | async/await, structured concurrency, channels/select, `parallel` | 🔴 |
| Stdlib | io/fs/net/http/json/csv/time/math/random/test/cli ≥ Python-parity table | 🟡 v0: json/file/random/time/math/text/list via `use` (B03+C06–C08) |
| Tooling | REPL, formatter, linter, LSP, debugger(DAP), test runner, doc generator | 🟡 CLI (run/parse/repl/version); rest 🔴 |
| Package ecosystem | semver registry, lockfile, workspaces, `project.nova` | 🔴 |
| Performance | fib/matmul/sort within 2× of C++ (LLVM), 100× of CPython on hot loops | 🔴 interpreter |
| Distribution | Single-file static executables; cross-compile matrix | 🔴 needs Python |
| Docs & learnability | Language reference complete, tutorial, playground, error index | 🟡 specs done + bootstrap cuts; tutorial missing |
| Unique differentiators | Flow, Table, track/undo, taint, state machines, time statements… | 🟡 track/undo + REPL-`:undo`; rest 🔴 |

## 3. Current status dashboard (v0.15-bootstrap — G0 closed; G1 in progress)

- ✅ G0 closed: suite green (223 tests), both examples perfect, error audit done (B01)
- ✅ Python bootstrap lexer/parser/tree-interpreter, Natural + shorthand skins
- ✅ Contracts (`requires` eager / `ensures` deferred), track/undo/redo
- ✅ Optional/`?` whole-expression poisoning; real modules with namespaces; stdlib v0;
  REPL; memory-model cut; **all diagnostics English**
- ❌ Result-typed errors (C04), lambdas/pipelines (C10), formatter/linter,
  native compiler — see backlog

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

**C03 — Optional/`?` semantics:** *(completed 2026-08-23 — kept as the worked example)*

- **Invariant 1 — one rule, sentence-shaped:** *if any part of an expression is
  `nothing`, and the expression carries a `?`, the WHOLE expression is `nothing`.*
  Not per-operand, not per-call — the entire enclosing expression.
- **Invariant 2 — without `?`, nothing still fails loudly** with the existing friendly
  error. `?` never silences logic errors: out-of-bounds, unknown fields on real things,
  unknown functions keep raising; ONLY absence-of-value propagates.
- **Trap — marker vs boundary.** Naïve postfix wrapping (`x? + 1` guarding only `x`)
  is WRONG. Parse-level transform: if the finished tree contains any `?`, strip the
  markers and wrap the ENTIRE tree once. Equivalence golden must pin this.
- **Scope fence:** `?` token + QuestionE + NothingSignal at exactly two throw-sites
  (Bin operand-guard, Field read of nothing). No Result type, no `try?`, no method chains.
- **Evidence required:** failing tests first incl. poisoning case; ≥1 cross-skin pair;
  golden 18-optional; suite green; interpreter diff limited to NothingSignal sites.

## 5. Phase gates

A phase is DONE only when every exit criterion holds. Gates are cumulative.

- **G0 — Bootstrap trustworthy** *(✅ CLOSED 2026-08-23 at v0.12)*: suite green; both
  examples perfect; error messages audited once end-to-end.
- **G1 — T1 language complete** (v0.13–v0.19): 100% of natural_syntax.md parseable &
  runnable; Result/Optional ✓; real module system ✓; stdlib-core v0 ✓; REPL ✓; golden
  AST dumps ✓; cross-skin equivalence harness ✓. Remaining: full natural_syntax coverage
  audit, C04, C10–C12.
- **G2 — Language credible** (v0.20–v0.29): `nova test` runner + assert lib; formatter
  v0 (idempotent); linter v0; deterministic `--sim`; docs site skeleton + tutorial;
  **freeze unique-feature design**.
- **G3 — Native pipeline alive** (v0.30–v0.49): **Rust** workspace + CI; M0 native
  lexer/parser byte-compatible with Python golden dumps; Nova IR + verifier; LLVM backend
  (via `inkwell`) runs guessing_game/todo natively; ARC pass; differential testing
  native-vs-bootstrap on the corpus.
- **G4 — Parity tooling** (v0.50–v0.69): VM bytecode + JIT-lite; LSP v1; DAP debugger;
  package manager + lockfile; stdlib parity-table v1; async/await + channels.
- **G5 — Differentiators shipped** (v0.70–v0.89): Flow<T>, Table(+SQL pushdown),
  history engine, state machines, time statements, `nova why`/`explain`, capability sandbox.
- **G6 — 1.0-ready** (v0.90+): freeze criteria §8 met; edition-2027 spec freeze;
  benchmark suite published; 10-app gallery incl. one self-hosted tool.

## 6. Phase backlog (live status — keep updated)

Statuses: ☐ Not started · ◐ In progress · ✅ Done (date) · ⏸ Blocked (by ID)

### Phase 0.2 — Native release track (owner-approved 2026-08-24; supersedes the old NEXT-UP QUEUE)
> **Contract:** v0.2 = first Rust-native interpreter runs real Nova programs end-to-end;
> Python bootstrap demoted to differential oracle. Ships as **v0.20.0**. Engine: bytecode
> compiler + stack VM behind a swappable-backend boundary (LLVM/JIT can slot in later).
> Types: dynamic core + opt-in annotations. Integers: arbitrary precision through 0.2.
> Audience: devs scripting/CLI tools → stdlib order cli/csv/datetime/regex.
> Uniqueness bets in 0.2: history/undo engine + Flow<T>. Tooling floor: nova test + LSP.

| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| N00 | Normative Natural-skin EBNF grammar (`specs/syntax/grammar_natural.md`); gap-audit vs goldens; closes Q12 | P0 | L | — | ✅ 2026-08-24 (chaining/`not`-asymmetry/try-body quirks pinned §6) |
| N01 | Rust workspace scaffold + skin-aware lexer; English messages ported from `nova_messages.py` | P0 | M | — | ✅ 2026-08-24 (token streams byte-identical to oracle on lexparity corpus + both examples; 19 unit tests; clippy-clean CI job added) |
| N02 | Rust parser → AST **byte-compatible with Python golden dumps** (= E02) | P0 | XL | N00, N01 | ✅ 2026-08-24 (all 20 goldens byte-equal; `nova parse` stdout identical to oracle on 22 files incl. both examples; C02 equivalence follows since dumps match) |
| N03 | Bytecode compiler + stack VM core: numbers/text/lists, control flow, functions (= E07 front half, pulled early) | P0 | L | N02 | ✅ 2026-08-24 (compiler.rs + frame VM: all loops/if-chains/functions+recursion/scope-chain writes/add-to/list aliasing/display rules; oracle-probed semantics; 40 Rust tests) |
| N04 | Runtime completeness: bigint, dicts/things, `nova_eq` pinning, Optional/`?`, contracts, modules, stdlib v0 parity | P0 | L | N03 | ✅ **2026-08-24 COMPLETE** (N04f-1 + N04f-2): check-patterns · try/if-it-fails cross-frame unwind · requires-hoisting + deferred ensures · things · track/undo/redo · phrase builtins (item/first/last/length/numval/random-between) · Optional `?` poisoning via signal handlers · compile-time string interpolation · **modules**: UseModule loader (cache/cycle-chain/isolated-env/mains-forbidden), ModuleCall into module programs, field reads of module vars · **stdlib v0**: json/file/random/time/math/text/list as native builtins with oracle-exact sentences |
| N05 | Differential harness vs Python oracle: corpus runner + output diff (= E06 early cut) | P0 | M | N04 | ✅ **2026-08-24: 29/29 identical** (`tests/native_diff.py`) — corpus extended with modules (basic/isolation/circular-error/missing-error), stdlib json/text/list/file/math, Optional-guard and deep-copy programs; hermetic temp-cwd runs (file-I/O safe). Known cosmetic divergences logged: native errors lack `column N`, oracle wraps with `Nova error —`; random excluded (different PRNGs by design) |
| N06 | Field stdlib pack: cli args/env/exit, csv, datetime parse+format, regex | P1 | L | N04 | ☐ |
| N07 | `nova test` runner (= D01) targeting the native CLI | P1 | M | N05 | ☐ |
| N08a | History/undo engine: design-freeze doc + v0 cut in native runtime | P1 | L | N05 | ☐ |
| N08b | Flow<T>: design-freeze doc + v0 cut (lists first; streams/channels stubbed) | P1 | L | N08a | ☐ |
| N09 | LSP v1: diagnostics, hover, completion (= F01 early) | P1 | XL | N02 | ☐ |

> Exit criterion for the phase: N00–N09 done, Python oracle suite green throughout,
> differential harness clean on the corpus → tag **v0.20.0**.

### Phase 0 — Bootstrap hardening (→ G0)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| B01 | Error-message audit: every NovaError = sentence + fix hint; catalog in tests | P0 | M | — | ✅ 2026-08-22 |
| B02 | Reserved-word policy: document builtin phrases that can't be variable names; enforce with clear error | P0 | S | — | ✅ 2026-08-22 |
| B03 | `use standard X` maps to real stub modules (random/json/file/time/math) instead of being ignored | P1 | M | — | ✅ 2026-08-23 |
| B04 | Unary minus + negative literals in expressions | P1 | S | — | ✅ 2026-08-22 |
| B05 | Golden AST dump tests: one file per construct under tests/golden/, compared via `parse` | P0 | M | — | ✅ 2026-08-22 |

### Phase 1 — T1 complete (→ G1)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| C01 | Compact-shorthand skin: lexer symbols, expression grammar; SAME AST nodes | P0 | L | B05 | ✅ 2026-08-22 |
| C02 | Cross-skin equivalence golden tests (Natural vs shorthand pairs) | P0 | M | C01 | ✅ 2026-08-22 (8 permanent byte-identical pairs; extend per construct) |
| C03 | `Optional<T>` values: `nothing` checks, `?` postfix propagation | P0 | M | — | ✅ 2026-08-23 |
| C04 | `Result` pattern: `try ... if it fails as err ... done` returns typed result; `give back ok/err` | P1 | M | C03 | ☐ folds in after the P0 spine (owner); target = post-v0.20 typed layer |
| C05 | Modules: `the tools-module in "tools.nova"` import; namespaces; circular-import error | P0 | M | — | ✅ 2026-08-23 |
| C06 | String library v0 | P0 | M | — | ✅ 2026-08-23 |
| C07 | List/dict library v0 | P1 | M | C06 | ✅ 2026-08-23 (map/filter/fold deferred to C10/T2) |
| C08 | Math/time/random library v0 per standard_library subset | P1 | S | B03 | ✅ 2026-08-23 |
| C09 | REPL: `nova repl` — persistent session, `:ast/:undo/:quit`, multiline via `done` | P1 | M | — | ✅ 2026-08-23 |
| C10 | Lambdas + pipeline `then` (T2 ergonomics over lists) | P2 | M | C07 | ☐ |
| C11 | match-exhaustiveness lite: `check` warns on missing otherwise (lint) | P2 | S | — | ☐ |
| C12 | Bootstrap perf pass: memoize dispatch tables; target 2× on todo bench | P2 | M | — | ☐ |
| C13 | Memory-model bootstrap cut: value/reference semantics pinned + `a copy of X` | P1 | S | — | ✅ 2026-08-23 (owner-prioritized outside §7 order) |

### Phase 2 — Credible language (→ G2)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| D01 | `nova test`: discovers `*.test.nova`, asserts lib, reports diffs | P1 | M | C05 | ✅ superseded by N07 (native CLI runner) |
| D02 | Formatter v0: token-stream based, idempotency property test | P1 | L | C01 | ☐ |
| D03 | Linter v0: naming, unused var, missing otherwise, truthiness trap | P2 | M | D01 | ☐ |
| D04 | `--sim` determinism mode | P2 | M | — | ☐ |
| D05 | Tutorial "Describe your first app" + docs site skeleton | P1 | M | — | ☐ |
| D06 | Unique-feature design freeze doc | P1 | M | — | ☐ |

### Phase 3 — Native pipeline (→ G3) — **Rust**
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| E00 | Native toolchain installed & verified; record decision in README log | P0 | S | — | ✅ 2026-08-23 (**Rust**: rustup 1.29, rustc/cargo 1.98.0; hello-world build+link+run verified) |
| E01 | GitHub Actions CI: Python suite on windows+ubuntu (fast feedback forever) | P0 | S | — | ✅ 2026-08-23 (.github/workflows/ci.yml; first run green in 39s) |
| E02 | Native lexer+parser (**Rust**), golden-dump byte-compatible with Python output | P1 | L | E00,B05 | ☐ |
| E03 | Nova IR (SSA) + verifier + text form `nova ir dump` | P1 | L | E02 | ☐ |
| E04 | LLVM backend via `inkwell`: enough integers/strings/lists to run guessing_game natively | P1 | XL | E03 | ☐ |
| E05 | ARC insertion pass + escape analysis | P1 | L | E04 | ☐ |
| E06 | Differential tester: run corpus through bootstrap & native, diff outputs | P0 | M | E04 | ☐ |
| E07 | VM bytecode backend + stack interpreter (REPL/scripting tier) | P2 | XL | E03 | ☐ |

### Phase 4 — Parity tooling (→ G4)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| F01 | LSP v1 | P1 | XL | E02 | ☐ |
| F02 | Debugger: DAP server on VM breakpoints/step/vars | P2 | XL | E07 | ☐ |
| F03 | Package manager: `project.nova`, resolve/lockfile, cache | P1 | L | C05 | ☐ |
| F04 | async/await + scheduler + Channel/select in native runtime | P1 | XL | E05 | ☐ |
| F05 | stdlib to parity-table v1 | P1 | XL | F03 | ☐ |
| F06 | Doc generator from `///` comments; error-code index (`nova explain`) | P2 | M | D05 | ☐ |

### Phase 5 — Differentiators (→ G5)
| ID | Item | P | Size | Depends | Status |
|---|---|---|---|---|---|
| H01 | History engine unification | P2 | L | F02 | ☐ |
| H02 | Flow<T>: one API lists/streams/channels | P2 | L | F04 | ☐ |
| H03 | Table primitive + CSV/JSONL + SQL-pushdown-lite | P2 | L | F05 | ☐ |
| H04 | Kernel state machines (`a workflow is ...`) | P2 | M | — | ☐ |
| H05 | Time statements `every day at 09:00` | P2 | M | F04 | ☐ |
| H06 | `nova why` + typed taint tracking | P3 | L | F01 | ☐ |
| H07 | Capability permissions `[permissions]` + sandboxed run | P3 | L | F02 | ☐ |

## 7. Priority list (flat, always sorted — work top-down skipping blocked)

> ### NEXT-UP QUEUE (owner-approved 2026-08-24 — the v0.2 native-release track, §6 Phase 0.2)
>
> Work top-down: **N00 → N01 → N02 → N03 → N04 → N05** (P0 spine), then
> **N06 → N07 → N08a → N08b → N09** (P1), then tag v0.20.0.
>
> *(Owner may reorder by editing this block — nothing outside it starts until it is empty or owner says go.)*
>
> **Status 2026-08-24:** N00–N05 ✅ complete — native modules + stdlib v0 at differential parity (29/29). Next: **N07** → **N06** (cli/csv/datetime/regex) → **N08a/b**; **N09 LSP moved to v0.21** (owner-approved).

1. ~~B05 golden dumps~~ ✅ · ~~B01~~ ✅ · ~~B02~~ ✅ · ~~C01+C02~~ ✅ · ~~C03~~ ✅ ·
   ~~C05~~ ✅ · ~~C06–C08~~ ✅ · ~~B03/B04~~ ✅ · ~~C09~~ ✅ · ~~E00+E01~~ ✅
2. **N-series 0.2 native-release track** — N00 first (see §6 Phase 0.2; supersedes the
   former D01→D05→D06→C04→C10→E02 queue; D05/D06/C04/C10 fold in after the P0 spine)
3. **D01/D05/D06** credibility pack (P1)
4. **C04/C10/C11/C12** (P1/P2)
5. **D02/D03/D04** (P1/P2)
6. **E02→E06** native pipeline sequence (P1)
7. **F-series** (P1/P2)
8. **H-series** (P2/P3)

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

| # | App | Proves | Gate | Status |
|---|---|---|---|---|
| A1 | Guessing game | teachability, I/O, loops | G0 | ✅ v0.11 |
| A2 | Todo (JSON persist) | CRUD, things, contracts, files | G0 | ✅ v0.11 |
| A3 | Notes CLI (args, dates, markdown files) | argv parsing, fs, time lib — **blocked on std.cli** | G1 | ☐ |
| A4 | CSV → stats report | strings, math, formatting — **blocked on csv** | G1 | ☐ |
| A5 | Mini web server | net/http, async | G4 | ☐ |
| A6 | Chat client+server | channels/select, structured concurrency | G4 | ☐ |
| A7 | Text adventure | kernel state machines (H04) showcase | G5 | ☐ |
| A8 | Data dashboard w/ undoable edits | Table + history engine | G5 | ☐ |
| A9 | `nova fmt` written IN Nova | self-hosting seed | G6 | ☐ |
| A10 | GUI notes app | GUI toolkit post-1.0 direction | post-1.0 | ☐ |

**Showcase rule:** every phase gate ends with a 60-second screen-recording of the newest
A-app running. If it can't be recorded, the gate isn't done.

## 10. Risks & mitigations + parking lot

| Risk | Mitigation |
|---|---|
| Scope creep (unique features tempting while core is young) | Hard rule §7: H-items locked until their gate; ideas → parking lot below |
| Two-skin divergence | C02 equivalence goldens run in CI forever |
| Bootstrap/native semantic drift | E06 differential tester at G3+; semantic pins (equality, division, phrase binding) already locked by tests |
| Solo-dev bus factor / motivation | Small items (S/M), weekly visible win, showcase recordings; **early differentiator demo recommended** (history engine) |
| Windows-only paths/encoding bugs | E01 CI on ubuntu from day one; UTF-8 enforced everywhere |
| Perf promises scare users away early | Bootstrap labeled reference-semantics oracle; perf marketing waits for G3 benchmarks |

**Parking lot (ideas, NOT commitments):** notebook/literate mode, blocks-editor for kids,
units/refinement types, actors/signals, GPU backend, grammar literals, `@incremental`,
list-slice/negative indices (C07 extension), `std.cli` argv+env+exit (blocks A3),
CSV reading (blocks A4), date/formatting in time-lib, string-interpolation-as-AST
(required by native E02 — today `{...}` is re-lexed at RUNTIME and invisible to goldens).

## 12. Open language decisions (owner must decide — from the 2026-08-23 critique)

| # | Question | Pressure | Status |
|---|---|---|---|
| Q1 | Language identity: English syntax + Danish errors/docs? | ~50 hardcoded strings; blocks public release | ✅ RESOLVED 2026-08-23: everything English |
| Q2 | Equality semantics unspecified (`true == 1`) | differential tester trap | ✅ RESOLVED 2026-08-23: pinned by tests + README log |
| Q3 | `divided` always returns float | native i32 divergence | ✅ DOCUMENTED 2026-08-23: real division now; int-div arrives with types |
| Q4 | Integer model: Python bigint now, promised i32 later | bootstrap/native divergence | ◐ documented in README; revisit at E02 |
| Q5 | Two ways to say "length" (`the length of` AND `text.length`) | violates design-law #2 | ✅ RESOLVED: phrases are primary, stdlib mirrors them (README log) |
| Q6 | `.name(...)` occupied by module calls — what about methods? | methods cannot be designed until resolved | ✅ RESOLVED: verb-first sentences (`finish t with ...`); dot stays module-only |
| Q7 | Silent-nothing vs loud-error split is accidental across builtins | unpredictability | ✅ RESOLVED: Ask/Act rule written + pinned by tests (error_handling §2.2) |
| Q8 | No LICENSE file | hard blocker for public release | ✅ RESOLVED: Apache-2.0 |
| Q9 | String interpolation re-lexed at runtime; invisible to goldens | E02 landmine | ◐ parked (interpolation-as-AST) |
| Q10 | No test-runner/formatter yet (D01/D02) | gates G2 | ☐ tracked as D-items |
| Q11 | When is `X is E` a declaration vs a comparison? The context rule is implemented but never written down | T1 learnability; blocks the natural-syntax coverage audit (G1) | ☐ |
| Q12 | No normative grammar doc for the Natural skin (`grammar.md` covers compact only; `natural_syntax.md` §3 is a sketch) | E02 native parser has no spec to build against — goldens are the de-facto spec | ✅ RESOLVED 2026-08-24: `specs/syntax/grammar_natural.md` is normative (N00); quirks pinned in its §6 |
| Q13 | Map phrase `set the age of X in M to V` exists in natural_syntax.md but is not implemented in bootstrap | spec/impl gap misleads learners; breaks "no dead ends" trust | ⏸ parked 2026-08-24: map phrase documented as future surface in natural_syntax.md; native dicts expose json-style access first |
| Q14 | Integer model: bootstrap bigint now vs promised i32 later | E06 differential tester will diverge on big literals / overflow | ◐ duplicate of Q4 - revisit with corpus design as N05 grows |

## 11. Changelog (newest first — mandatory updates)

| Date | Change |
|---|---|
| 2026-08-24 | **N04f-2 ✅ + N05 extension**: native modules (loader w/ cache + circular-chain error + isolated per-program env + mains-forbidden) and stdlib v0 (json/file/random/time/math/text/list) at oracle parity; Value gained Dict(insertion-ordered)/Module; frames carry their own Program+env (exec_until_depth); GetField reads module vars; CopyOf rejects modules; float render trims integral .0. Harness grew 18→29 incl. hermetic file-I/O runs; prefix-stripper fixed for leading newline. Validation: cargo test/clippy clean, oracle 236/236, diff 29/29.
| 2026-08-24 | **N04a–e ✅ + N05-lite ✅ (native runtime wave)**: check-patterns; try/if-it-fails via handler-stack + manual frame unwinding (step() dispatch refactor — errors recover mid-loop, message text bound w/o line prefix per oracle); requires hoisted above body even when written last + ensures evaluated at every exit via @ret slot; ThingDef compiled as `@new:<cls>` ctor func (defaults incl. nothing), setters = call+Dup+StoreField chain, identity eq, deep copy, `dog(...)` display; track/undo/redo as state-stack history (snapshot at track, push on rebind, undo↔redo stacks, fresh change clears redo). **N05-lite:** tests/native_diff.py — 18/18 corpus programs byte-identical vs oracle. Suite: 51 Rust tests + 236 Python + clippy clean. Remaining N04f: modules/stdlib/optional/phrases. |
| 2026-08-24 | **N03-fix**: VM dispatch made fully iterative (frames on heap, no Rust recursion) after stack-overflow at fib(10); Nova recursion now heap-bound. CLI gained `nova run`; examples/native_demo.nova added (runs natively). |
| 2026-08-24 | **N03 ✅ — native bytecode+VM runs real programs**: statement compiler (if-chains with jump patching, all 6 loop forms incl. skip/stop, FuncDef pre-declaration, CallName for runtime func-not-found), frame-based VM (scope-chain writes: local→global→create-in-current; oracle-pinned via probes), iterators for list/text/range, `render()` display rules (lowercase bools, `nothing`, `[a, b]`), AddTo list-append/num-increase. Recursion verified (fib(10)=55). 40 Rust tests green + clippy clean. Remaining for N04: check/things/modules/contracts/track-undo/try/stdlib. |
| 2026-08-24 | **N03a ✅ (VM expression core)**: `value.rs` — Value model with bigint via num-bigint, `nova_eq` ported exactly (bools≠numbers, int/float cross, structural lists), Python-sign modulo; `bytecode.rs` — stack Instr set incl. JumpIfFalse/JumpIfTrue/MustBeBool/MustBeList; `vm.rs` — stack machine with oracle sentence errors. Discovered: oracle CRASHES raw on ordering non-numbers (`'<' not supported`) — native emits a proper sentence; align at N05. 31 Rust tests green. |
| 2026-08-24 | **N02 ✅ — NATIVE PARSER AT ORACLE PARITY**: full Rust port of nova_parser.py (all statements, expression chain, check/try/contracts/modules, `?` whole-expression wrap). Golden harness (`native/nova/tests/golden.rs`): all 20 dumps byte-for-byte. CLI `nova parse` stdout identical to `nova_cli.py parse` on 22 files. E02 satisfied early. |
| 2026-08-24 | **N01 ✅**: native Rust workspace (`native/nova`) — skin-aware lexer ported 1:1 from bootstrap (hyphen policy, BOM/shebang, escapes, arbitrary-precision ints as strings, float rule, `;`-newline, symbol skins). Differential spot-check: token streams byte-identical to Python oracle on a 110-token torture file + guessing_game + todo (593 tokens). Message catalog: shared+lexer+parser groups byte-equal. CLI `nova version`/`nova lex`. CI rust job (test+clippy `-D warnings`). |
| 2026-08-24 | **N00 ✅**: normative Natural-skin grammar written (`specs/syntax/grammar_natural.md`) — full EBNF extracted from the bootstrap parser, dump-contract node table, verified quirks pinned (comparison-tail chaining incl. `is`-reintroduction rule, `not`-vs-`!` operand asymmetry, try-body `if` limitation, optional middle `of`). Q12 closed. |
| 2026-08-24 | **v0.2 native-release contract locked (owner)**: N-series track added (§6 Phase 0.2) — Rust bytecode+stack VM behind swappable backend, dynamic + opt-in annotations, bigint through 0.2, audience = dev scripting/CLI, stdlib pack cli/csv/datetime/regex, uniqueness bets history/undo + Flow<T>, tooling floor nova-test + LSP, grammar-doc-first (N00). Old D01→E02 queue superseded; ships as v0.20.0. |
| 2026-08-23 | **i18n docs sweep completed (for real)**: EXTENSIONS, language_reference, unique_features, standard_library, module_system, concurrency, memory_model, grammar → 100% English (grep-audited). Owner decisions: `it` reserved (protects check/try patterns; +2 tests → suite **236/236**), open questions Q11–Q14 added to §12. |
| 2026-08-23 | **E01 ✅ + repo LIVE**: github.com/Carlingmarfred/nova public (Apache-2.0); CI green first run (windows+ubuntu, 39s). Q5/Q6/Q7/Q8 decisions resolved. v0.15.1 tagged. Suite: 234/234. Next: D01/D05/D06 → C04/C10.
| 2026-08-23 | **Decisions landed (v0.15.1)**: Apache-2.0 LICENSE added; E01 CI workflow created (.github/workflows/ci.yml, windows+ubuntu); Q5 phrases-primary, Q6 verb-first methods, Q7 Ask/Act rule (+11 pin tests), Q8 license — all resolved in README log + §12. Suite: 234/234.
| 2026-08-23 | **v0.15.0-bootstrap**: full English documentation sweep completed (12 specs + ARCHITECTURE/ROADMAP/EXTENSIONS/README/AGENTS/notes — zero Danish lines remain anywhere); version bump; E00 Rust recorded.
| 2026-08-23 | **fix commit dd853ca**: modulo-by-zero guard (was raw traceback); equality semantics pinned via `nova_eq` (bools≠numbers, structural lists/dicts, identity things) applied to eq/ne/check/take/contains; first/last/length/count operands bind at factor level. +12 tests. Suite: 223/223. |
| 2026-08-23 | **i18n commit 70b0464**: ALL runtime diagnostics translated to English (lexer/parser/interpreter/cli/repl), reserved-word `what=` args included; every Danish test assertion updated; `nova_messages.py` kept as reference catalog. Suite: 211/211. |
| 2026-08-23 | **E00 ✅ — Rust chosen** (owner decision): rustup 1.29 / rustc+cargo 1.98.0 installed and verified (build+link+run). ROADMAP/ARCHITECTURE updated to Rust pipeline. README decision log records the choice. |
| 2026-08-23 | **docs-audit**: stale claims fixed (shorthand implemented, known-holes rewritten, counts). lab/unique/tour verified failing cleanly. |
| 2026-08-23 | **C09 ✅**: `nova repl` — persistent session, expression echo, `:ast/:undo/:quit/:help`, multiline via done. Spec: ARCHITECTURE §10. 8 tests. |
| 2026-08-23 | **C13 ✅** (owner-prioritized): memory-model bootstrap cut — value/reference semantics pinned, `a copy of X` deep copy; golden 20; pair 8. |
| 2026-08-23 | **B03+C06+C07+C08 ✅**: stdlib v0 trio complete (json/file/random/time/math/text/list). |
| 2026-08-23 | **C05 ✅**: modules with namespaces, paren calls, circular-import error; golden 19; pair 7. |
| 2026-08-23 | **C03 ✅**: Optional/`?` whole-expression poisoning; golden 18; pair 6; NumVal factor-binding; plus type-mismatch sentence. |
| 2026-08-22 | **C01+C02+B04 ✅** shorthand skin + equivalence pairs + unary minus (132/132). **B05/B01/B02 ✅** goldens, error audit, reserved words. |
| 2026-08-22 | Plan created at v0.11-bootstrap; baseline 49/49 tests. |
