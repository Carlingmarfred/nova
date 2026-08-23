# Nova Error Handling

## 1. Philosophy

- **Expected errors** (file missing, network down, bad input) = values: `Result<T,E>`,
  `Optional<T>`.
- **Unrecoverable errors** (broken invariant, OOM, index errors) = panic (unwind or abort).
- Exceptions (`try/catch`) exist but are panic handling — never control flow. The linter
  flags catch in normal flow logic.

## 2. Optional<T> ≡ T?

```text
find_user(id) -> User?

u = find_user(42)
if u == none { return }
print(u.name)                 # flow typing: u is User here

u?.email?.primary             # chaining → String?
name = u?.name ?? "anonym"    # coalescing
forced = u!                   # unwrap-or-panic (only with good reason; lint)
```

Flow-sensitive typing: after `if x == none { return }` x is known non-none. Also holds
after `?`, assert, match.

### 2.1 Bootstrap cut (v0.11+, item C03) — IMPLEMENTED in bootstrap v0.12

The bootstrap implements one precise sub-rule of `?` — **whole-expression poisoning**:

> If ANY part of an expression is `nothing`, and the expression carries a `?`, the WHOLE
> expression becomes `nothing`. Without `?` it fails as always with a friendly message.

```text
n = the number value of answer? + 1     # answer = "abc"  →  n = nothing (no crash)
v = the number value of "41"? + 1       #                  →  v = 42
say "{the text of maybe?}"              # maybe = nothing  →  prints nothing
```

Precise rules (C03 contract, ITERATION_PLAN §4.5):
1. **Whole-expression poisoning — one rule, sentence-shaped:** if any part of an
   expression is `nothing`, and the expression carries a `?`, the WHOLE expression is
   `nothing`.
2. **Marker position is free:** `q? plus 1` and `q plus 1?` are the SAME expression.
   At parse time all `?` markers are stripped and the finished tree is wrapped exactly
   once (`QuestionE`). Dumped ASTs never contain nested markers — only one root wrapper
   (pinned by golden 18-optional and cross-skin pair 6).
3. **ONLY absence-of-value propagates** — exactly two throw-sites in the interpreter:
   - arithmetic/ordering operations on `nothing` (`plus/minus/times/divided/mod`,
     `gt/gte/lt/lte`),
   - field reads of `nothing` (`the text of maybe?` / `maybe.text?`).
   Out-of-bounds (`item 9 of xs`), unknown fields on REAL things, unknown functions and
   invalid json still FAIL — even under `?`. `?` never covers logic errors.
4. **Equality with `nothing` is NEVER poisoned** — that IS the test:
   `if x is nothing then ...` / `if x is not nothing then ...` work as always, even when
   `x` is `nothing` (eq/ne are exempt from rule 3).
5. **Without `?` it still fails loudly** with a friendly sentence + fix hint ("cannot do
   arithmetic on 'nothing' — add '?' ... or check the value with 'is nothing' first").
   Is `NothingSignal` caught by `try ... if it fails`? NO — absence is not an error;
   only the `QuestionE` boundary swallows the signal.
6. Results are tested with the existing checks from rule 4; `say "{...}"` shows `nothing`
   as text. Applies to both skins and to string interpolation `{...}`.

### 2.2 The absence-vs-error rule (decided 2026-08-23, owner Q7)

Every built-in falls into exactly one of two families — no exceptions, no judgment calls:

| Family | Behavior on "nothing here" | Members |
|---|---|---|
| **Ask** — asks a question about a value | returns `nothing` | `the first item of []`, `the last item of []`, `the number value of "abc"` |
| **Act** — acts on an expectation | raises a friendly NovaError | `item N of` out of bounds · unknown variable/field-on-real-thing/function · file missing · invalid json · non-sized `length`/`how many` · wrong-type `contains` · arithmetic or field read on `nothing` without `?` |

Corollaries:
1. `?` exists for Ask-family values flowing through Act-family operations: it turns
   those specific absences into `nothing` instead of a raise (C03).
2. Adding a new builtin requires declaring its family in this table (test-enforced:
   `tests/run_tests.py::test_nothing_rule` pins every member).

## 3. Result<T,E>

```text
fn read_config(path: String) -> Result<Config, ConfigError> {
    text = File.read(path)?            # Err propagates to the caller
    parsed = json.parse(text)?         # IoError → ConfigError: auto-conversion via From trait
    Config.from_json(parsed)
}
```

Combinators:

```text
r.map(|v| v * 2)
r.and_then(validate)                   # flat_map
r.or_else(fallback_fn)
r.unwrap_or(default)  r.unwrap_or_else(gen)
r.expect("config was required")        # panics with message on Err
r.ok()                                 # → Optional
result_tuple.unzip()
```

Error types: any type can be E; stdlib uses the hierarchy `Error` base + specific types;
auto-conversion via `From<E2> for E1`.

Bootstrap note (C04 pending): today's `try ... if it fails as err` binds err to a plain
text message; typed results arrive with C04.

## 4. The ? operator — precise rules (full language)

- `expr?` where expr: `Result<T,E>` → `T` in the Ok case; `return Err(e.into())` otherwise.
- `expr?` where expr: `T?` → `T`; `return none` otherwise.
- Only valid in functions returning compatible `Result/_?` (or inside `try { }` blocks).
- Chaining: `File.open(p)?.read()?.parse()?`.

## 5. Panic

```text
panic("impossible state: {}", state)
assert(cond, "message")          # debug-only
require(cond, "message")         # always active
unreachable()                    # debug: panic; release: UB-marked (lint)
todo("not implemented")
```

Panics unwind the stack (destructors/defer run) until:
- process boundary (default): message + backtrace + exit code 101
- nearest `catch`

## 6. try / catch / finally

```text
try {
    run_plugin(untrusted_input)
} catch e: PluginError {
    log.warn(e)
} catch e {                          # all other panics
    log.error(e.backtrace)
} finally {
    cleanup()
}
```

Use areas: plugin interfaces, FFI boundaries, top-level crash handlers, benchmarks.
Catch of `dynamic` errors for dynamic code.

## 7. Backtraces and diagnostics

- Panics print file:line + symbolized backtrace (DWARF/PDB) in debug; `NOVA_BACKTRACE=full`.
- `Result.Err` can carry `.trace` (capture optional, `--error-trace`).
- Structured compiler diagnostics: code (E0432-style), span, notes, fixes
  (machine-applicable for LSP).

## 8. Interop

- C: errno/errno-style status → `Result` via bindings generator.
- Python: exceptions → `Err(PyError)` at the boundary.
- JVM: checked/unchecked exceptions → `Result<_, JavaThrowable>` in bridge APIs.
