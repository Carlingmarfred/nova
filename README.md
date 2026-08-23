# Nova

A general-purpose programming language: **C++ performance + Python simplicity +
Java ecosystem** — built around one strong kernel (compiler + typesystem + IR +
runtime) with every other feature layered on top.

> **Status: v0.15-bootstrap** — the Python interpreter in `bootstrap/` is green on
> the full end-to-end suite (`python tests/run_tests.py`, 223 tests incl. both
> examples). **Gate G0 is closed; G1 (T1 complete) is in progress.** Implemented:
> compact shorthand skin, Optional/`?`, real modules with namespaces, stdlib v0
> (`use the standard X library`: json/file/random/time/math/text/list), an
> interactive REPL (`python bootstrap/nova_cli.py repl`), and the memory model's
> value/reference semantics with `a copy of X`. All diagnostics are English.
> See [project-notes.md](project-notes.md) §5.

```
# Nova Natural (primary syntax — reads like sentences):
when the program starts
    secret is a random number between 1 and 100
    repeat until the guess is the secret
        answer is ask "Your guess: "
        if answer is not a number then say "That is not a number."
        otherwise if guess is less than secret then say "Higher!"
        otherwise if guess is greater than secret then say "Lower!"
        otherwise
            say "Correct! The number was {secret}."
            stop the loop
        done
    done
done

# Compact shorthand (same AST — expert stenography):
x = 10                          # inference -> i32, mutable binding
nums = [1, 2, 3].map(x => x * 2).filter(x => x > 4)
fn read(path: String) -> Result<String, IoError> { Ok(File.read(path)?) }
```

## Documentation

| Document | Contents |
|---|---|
| [docs/ITERATION_PLAN.md](docs/ITERATION_PLAN.md) | **Living plan: workflow, priorities, phase gates, 1.0 criteria, status/changelog — always current** |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Compiler pipeline, Nova IR, backends, runtime, VM, tooling |
| [specs/natural_syntax.md](specs/natural_syntax.md) | **Nova Natural: the English sentence syntax** — full vocabulary table + desugaring |
| [specs/language_reference.md](specs/language_reference.md) | Complete language reference: all constructs, operators, collections |
| [specs/syntax/grammar.md](specs/syntax/grammar.md) | Formal EBNF grammar |
| [specs/syntax/lexical.md](specs/syntax/lexical.md) | Tokens, literals, keywords, operators |
| [specs/type_system.md](specs/type_system.md) | Static/dynamic typing, inference, generics, traits, unions |
| [specs/memory_model.md](specs/memory_model.md) | ARC default, ownership, unsafe, GC profiles |
| [specs/error_handling.md](specs/error_handling.md) | Result/Optional/`?`, panics, exceptions |
| [specs/concurrency.md](specs/concurrency.md) | async/await, parallel, channels, select, structured concurrency |
| [specs/metaprogramming.md](specs/metaprogramming.md) | Compile-time eval, derive macros, reflection |
| [specs/module_system.md](specs/module_system.md) | Modules, packages, `project.nova`, dependency handling |
| [specs/standard_library.md](specs/standard_library.md) | Full stdlib surface with a Python-parity table |
| [examples/tour.nova](examples/tour.nova) | The whole language in one commented program |
| [docs/EXTENSIONS.md](docs/EXTENSIONS.md) | 16 proposed extensions: units, signals, actors, contracts, time-travel debug etc. |
| [specs/unique_features.md](specs/unique_features.md) | **The 13 unique features only Nova has**: Flow, Table, undo/history, taint, state machines and more |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Milestones M0–M8 |

## Decision log (open questions answered)

| Question | Decision | Rationale |
|---|---|---|
| Native implementation language | **Rust** (owner decision 2026-08-23). Toolchain installed and verified (rustc/cargo 1.98). LLVM via `inkwell` when E03/E04 start. | Memory safety without a borrow-checker on our own IR passes, best-in-class LLVM bindings, faster iteration than C++ for a solo dev. Replaces the earlier C++ assumption throughout ROADMAP/ARCHITECTURE. |
| Human language of docs & diagnostics | **English everywhere** (owner decision 2026-08-23). Example *programs* may keep Danish UI text for now. | International release ambition; centralized message catalog (`bootstrap/nova_messages.py`) keeps future localization cheap. |
| Memory model | **ARC as default** + compile-time escape analysis (removes retain/release pairs where possible). `owned` opt-in, `unsafe` raw pointers. GC only via `--runtime full`; cycles handled by a cycle collector in the full runtime; minimal profile requires `weak`. Bootstrap cut (C13): value/reference semantics pinned + `a copy of X` deep copy. | Deterministic deallocation, implementable as an early IR pass, proven by Swift. Full GC deferred without blocking the frontend. |
| Equality semantics | **Pinned by tests (2026-08-23):** bools are never equal to numbers; numbers compare across int/float; text/lists/dicts compare structurally; things/functions/modules compare by identity. | Removes a Python-behaviour leak (`true == 1`); differential tester (E06) needs this fixed now. |
| Dynamic typing | `dynamic` = complete runtime system (tagged value + inline caches in VM / vtable native). Specialization is a later optimization. | The kernel must work 100% dynamically first; specialization is additive. |
| C++ interop | **C ABI first** + automatic header import for C. True C++ interop via generated shims later. The ABI never assumes C++ semantics (no exceptions/RTTI in the ABI). | Full C++ interop is a project the size of the compiler itself. |
| Java interop | No JVM backend in v1. Bridge through a JNI-like interface over the C ABI. JVM backend revisited when Nova IR is stable. | A JVM backend locks the IR design to Java's typesystem (no unsigned ints, no value types). |
| Runtime | Two profiles from day 1: `--runtime minimal` (ARC core, no scheduler/reflection data) and `--runtime full`. Stdlib core never calls the runtime directly, only through capability interfaces. | Forces clean layering; makes embedded/single-file builds possible. |
| Concurrency | `parallel` = compiler-managed (work-stealing task scheduler), plus explicit primitives (`spawn`, `Channel`, `Mutex`, `select`) for full control. Structured concurrency as a ground principle. | The two levels are complementary; `parallel` is syntactic sugar over the task system. |
| Syntax | **Nova Natural as primary syntax**: common English words in fixed phrases — `say`, `ask ... and remember it as`, `set x to`, `if ... then ... otherwise ... done`, `repeat until ... done`, `to greet with name ... done`. Blocks terminate with `done` (unambiguous, no indentation sensitivity). Compact shorthand (`{}`, `=>`, symbol operators, `x = 10`) stays valid expert form — **both skins compile to identical ASTs**. Explicit types required on public API signatures (compact form). | Owner's core requirement: code should read word-for-word like explaining the idea to a human. `done` terminators avoid JS-style ASI errors and Python indentation fragility; the two-skin design keeps full expressive power without duplicating semantics. |
| Phrase operand binding | Built-in value phrases (`the number value / first item / last item / length of ...`, `how many items are in ...`) bind their operand at **factor level** (2026-08-23). Phrases with trailing keyword clauses (`contents of X parsed as json`, `every item of X turned into a T`) stay greedy. | Greedy binding made `(nv x?) + 1` impossible and broke arithmetic composition; factor binding matches operator-precedence intuition while trailing-clause phrases need greediness to consume their keywords. |
| Extensions | **All 16 extensions approved**: refinement types, units (dimensional analysis), verified format strings, pipelines/`then`, signals, actors, contracts, capability permissions, reproducible builds, time-travel debugger, notebook/literate mode, `nova explain`, API diff, teaching pack + block editor, embedding API, native hot reload. Integrated into the kernel specs — see [docs/EXTENSIONS.md](docs/EXTENSIONS.md). | Extends coverage from "can do anything" to "has the best tool for it"; all build on the kernel without changing it. |
| Unique features | **13 features that make Nova unique** (see [specs/unique_features.md](specs/unique_features.md)): Flow<T> (one API for lists/streams/channels), Table as a language primitive with SQL pushdown, undo/redo + variable-history queries (`track`/`undo`/`ever`), typed taint tracking, state machines in the kernel, `exact` math blocks, deterministic sim-test standard, `@incremental`, time expressions (`every day at 09:00`), `nova why`, grammar literals and the **pure-Nova stack** (stdlib depends only on OS syscalls — own regex/TLS/db/compression). | Owner requirement: features no other language has — without sacrificing readability. Honest per-feature comparison table lives in the spec; the uniqueness lies in the integration: one history engine drives undo + debugging + revisions, one Flow engine drives iterators + streams + channels. |

### Further settled kernel decisions

- **String**: UTF-8, immutable `String` + `StringBuilder`; `char` = Unicode scalar value.
- **Integers**: fixed-width default (`i32` inferred for int literals); overflow check in
  debug, wrapping in release (opt-in `@checked`). `BigInt` in stdlib. *Bootstrap note:
  the Python interpreter uses arbitrary-precision integers until M1.*
- **Floats**: IEEE-754 `f32`/`f64`; `Decimal` and `Rational` in stdlib. *Bootstrap note:
  `divided` is always real division (`7 divided by 2` = `3.5`); integer division arrives
  with the native type system.*
- **Error model**: `Result<T,E>`/`Optional<T>` primary; `throw`/`try-catch` = panic handler
  for exceptional cases, never control flow.
- **Contracts (v0.11)**: `requires` evaluated eagerly at call; `ensures` deferred to
  function exit and evaluated in the function-local scope — postconditions can reference
  the final state. Failure → `NovaError`, catchable with `try ... if it fails`.
- **Type unions**: anonymous sum types allowed: `i32 | String`.
- **File extension**: `.nova`.

## Core principle

> Nova does not implement Python, Java and C++ as three systems. One kernel —
> compiler, typesystem, IR, runtime — then Python-dynamism, Java-reflection and
> C++-control layered on top of that same kernel.
