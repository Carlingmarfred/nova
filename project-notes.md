# Nova — Project Notes

> **Status v0.14-bootstrap:** Gate G0 CLOSED; G1 in progress. B05/B01/B02 +
> C01/C02/B04 + C03/C05/C09 + B03+C06+C07+C08 stdlib v0 + docs-audit + i18n-to-English
> + C13 memory cut + semantic-equality pinning done; **223/223** end-to-end tests green
> (`python tests/run_tests.py`). Changelog in §5.

## 1. Context & Goals

Building **Nova**, a new general-purpose programming language: C++ performance + Python
simplicity + Java ecosystem. Session history: full design spec suite → natural English
syntax pivot → 16 extensions approved → 13 "unique-only-to-Nova" features approved →
**implementation started** (Python bootstrap interpreter, `bootstrap/`). Owner directive
2026-08-23: **decouple from Python — native pipeline is written in Rust** (toolchain
installed); all documentation and diagnostics are English.

Repo layout (working dir `oxtest/`):

- `AGENTS.md` — working agreement for agent sessions (read first)
- `README.md` — overview + full decision log (English)
- `docs/` — ITERATION_PLAN.md (living plan, always updated), ARCHITECTURE.md,
  ROADMAP.md, EXTENSIONS.md
- `specs/` — language_reference, natural_syntax, type_system, memory_model,
  error_handling, concurrency, metaprogramming, module_system, standard_library,
  unique_features, syntax/grammar.md, syntax/lexical.md
- `examples/` — tour.nova, guessing_game.nova, todo.nova, lab.nova, unique.nova
- `bootstrap/` — Python interpreter: nova_lexer.py, nova_parser.py,
  nova_interpreter.py, nova_messages.py (reference catalog), nova_cli.py,
  nova_dump.py (v0.14: 223/223 green via tests/run_tests.py)
- `tests/` — run_tests.py end-to-end suite (subprocess-based)

## 2. Key Decisions

**Language core (decision log in README.md):**
- Native pipeline: **Rust** (2026-08-23); LLVM via inkwell at E03/E04
- Memory: value/reference semantics pinned (C13) + `a copy of X`; ARC default later (E05)
- Equality pinned (2026-08-23): bools never equal numbers; int/float cross-compare;
  structural text/list/dict; identity for things/functions/modules (`nova_eq`)
- Division: always real division; integer division arrives with the type system
- `dynamic` = complete runtime system; interop C ABI first; runtime profiles minimal/core/full
- Concurrency: `parallel` = compiler-scheduled tasks + explicit primitives
- Strings UTF-8; errors Result/Optional primary; contracts requires-eager/ensures-deferred

**Syntax (Nova Natural is PRIMARY):**
- English phrases: `say`, `ask`, `set x to`, `add n to`,
  `if ... then ... otherwise ... done`, `repeat until/forever/N times/for each ... done`,
  `to greet with name ... done`, `when the program starts ... done`,
  `check x / when it is ...`
- Blocks terminated with `done` (no indentation sensitivity). Statements end at newline.
  Inline `then` bodies are single-statement and cannot be followed by a block `otherwise`
  on later lines — use full block form when any branch is multi-line.
- Field access ALWAYS `the <field> of <obj>` (chainable); dotted `x.f` also works and is
  attribute access (reserved words do NOT apply after `.`).
- Compact symbol syntax shares ONE expression grammar → identical ASTs (C01/C02).

**Approved feature packs:** 16 extensions (docs/EXTENSIONS.md); 13 unique features
(specs/unique_features.md).

**Implementation strategy:** Python 3.x bootstrap first as reference oracle; native
pipeline in **Rust** per ROADMAP M0+; differential testing (E06) keeps them honest.

## 3. Actionable Commands & Code Snippets

```powershell
python tests/run_tests.py
python bootstrap/nova_cli.py run examples/guessing_game.nova --seed 7
python bootstrap/nova_cli.py repl --seed 42
python bootstrap/nova_cli.py parse examples/todo.nova   # AST dump (golden tests)
python bootstrap/nova_cli.py version                     # Nova 0.14.0-bootstrap
```

**Parser invariants (bootstrap/nova_parser.py):**
- Statements do NOT consume trailing NEWLINE — block loops own newline skipping.
- `p_body(stop_words)` returns `(Block, used_done)`; inline bodies end at NEWLINE.
- `the F of OBJ` chains built in `parse_the_chain()`. Generic postfix `of` removed.
- **Value phrases bind operands at FACTOR level** (v0.12+/fixes): `the number value of`,
  `the length of`, `the first/last item of`, `how many items are in`. Phrases with
  trailing keyword clauses stay greedy: `contents of X parsed as json`,
  `every item of X turned into a T`.
- RESERVED_WORDS frozenset; every name-binding site goes through
  `expect_name(what)` with ENGLISH what-strings ("variable name", "field name", ...).
- `my x is E` is a valid declaration (sugar).
- **Golden AST dumps (B05):** canonical format in nova_dump.py header; native M0 must
  match byte-for-byte. After deliberate grammar changes run
  `python tests/run_tests.py --update-goldens` and review the diff.
- **Compact skin (C01):** lexer `_SKIN_SINGLE/_SKIN_DOUBLE`; two-char tokens before
  single. Hyphen policy: `-` + letter continues a word; `-` + digit = MINUS.
  Parser chain: comparison → arith → term → factor(unary ! -) → postfix(.field ?) →
  primary. Symbol ops reuse EXACT word-skin op strings; unary minus desugars to `0 - x`.
- **Modules (C05):** `p_usemodule()` requires names ending `-module`; `ModuleCall`
  only when base is bare Var; other bases keep the friendly "later version" error.
- **Optional (C03):** `parse_postfix` consumes QUESTION markers;
  `wrap_optional()` strips ALL markers and wraps the whole tree ONCE at the root
  (applies to `{...}` interpolation too). Dumped ASTs never contain nested markers.

**Interpreter notes (bootstrap/nova_interpreter.py):**
- Control flow via exceptions: Break/Continue/Return/ExitSignal + NothingSignal (C03).
- **Equality (pinned):** `nova_eq(a,b)` — bools strict-typed vs numbers; int/float cross;
  structural str/list/dict; identity for things/functions/modules. Used by eq/ne,
  check-patterns, `take ... from` membership and list `contains`.
- **Optional:** NothingSignal thrown at exactly two sites (arith/ordering Bin guard,
  Field read of nothing); QuestionE eval is the only catch-site; CLI converts uncaught
  signals to sentence errors. eq/ne with nothing never poisons.
- **Stdlib (B03):** `use [the] standard NAME [library]` binds NAME to ModuleInstance of
  BuiltinFunctions via STDLIB_FACTORIES (cached). Dotted names skip reserved-word checks.
- **Memory (C13):** lists/things/dicts are references; numbers/text/bool values;
  `a copy of X` → CopyOf node → copy.deepcopy (modules/functions rejected politely).
- **REPL (C09):** `_run_repl()` — persistent Interp; 'done'-family parse errors keep
  buffering; non-statement lines echo as `→ value`; `:undo` restores deep-copied
  globals/funcs/things (max 100).
- `plus` with mixed number/text = friendly NovaError (was raw TypeError).

## 4. Next Steps

1. ✅ v0.11 next-steps complete (see §5 changelog history)
2. **E01**: GitHub Actions CI (windows+ubuntu) — P0, next up
3. D01/D05/D06 credibility pack
4. C04 Result pattern; C10 lambdas/pipelines
5. E02+: native Rust lexer/parser matching golden dumps byte-for-byte

## 5. Changelog (v0.11 →)

**2026-08-23 — fixes (dd853ca, 223/223):** modulo-by-zero guard; `nova_eq` semantic
pinning applied everywhere (bool≠number leak fixed incl. `take`/`contains` membership);
first/last/length/count bind operands at factor level (greedy parse made arithmetic
composition impossible). +12 tests.

**2026-08-23 — i18n (70b0464, 211/211):** all diagnostics English; lexer rewritten on
`nova_messages` catalog; parser/interpreter/cli translated (~120 sites); reserved-word
`what=` args English; all test assertions updated.

**2026-08-23 — C09 REPL (199/199):** persistent session; expression echo with
statement-parse fallback; meta commands; undo stack.

**2026-08-23 — C13 memory cut (211/211):** aliasing pins; `a copy of X`; golden 20;
pair 8. Note: `add`/`set` targets are NAMES (`item` reserved there) — bind inner lists
before mutating.

**2026-08-23 — stdlib v0 (170→191):** B03 mechanism + text + list + math/time/random
fill-outs; dotted names exempt from reserved words; ContentsOf reuses _read_text_file.

**2026-08-23 — C05 modules (159/159):** import statement, namespaces, paren calls,
circular-import detection (stack BEFORE cache), arity shared via `_invoke`, CLI catches
module parse errors at run time.

**2026-08-23 — C03 Optional (146/146):** QUESTION token; wrap_optional root-wrapping;
NothingSignal two throw-sites; NumVal factor-binding; plus type-mismatch sentence.

**2026-08-22 — v0.11 (49→132):** goldens + error audit + reserved words + shorthand
skin + equivalence pairs + unary minus.

**Known gaps in v0.14 (updated 2026-08-23):** closed so far — shorthand ✓, unary minus ✓,
Optional ✓, modules ✓, stdlib v0 ✓, REPL ✓, memory semantics ✓. STILL OPEN: typed Result
(C04), lambdas/pipelines (C10), check-exhaustiveness lint (C11), formatter/linter
(D02/D03), LICENSE file (owner decision Q8), tour/lab/unique.nova outside bootstrap scope
(fail cleanly — verified), spans only in frontend errors, built-in phrase heads win over
same-named field names in `the ... of` form (use dotted access).

## 6. Original Next Steps (historical)

1. Fix guessing_game `done` structure ✅
2. Rewrite todo.nova to parser-compatible Natural ✅
3. Create tests/run_tests.py ✅
4. Run tests green ✅
5. Later: native pipeline per ROADMAP → now **Rust** (M0 lexer/parser + golden dumps)
