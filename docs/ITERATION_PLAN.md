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
| C04 | `Result` pattern: `try ... if it fails as err ... done` returns typed result; `give back ok/err` | P1 | M | C03 | ⏸ deferred: typed layer lands after v0.20 (owner decision 2026-08-24); bootstrap keeps message-binding errors until then |
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
| D01 | `nova test`: discovers `*.test.nova`, asserts lib, reports diffs | P1 | M | C05 | ✅ superseded by N07 (native CLI runner - owner 2026-08-24) |
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

