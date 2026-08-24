# Nova Architecture

## 1. Pipeline overview

```text
Source (.nova)
    │
    ▼
┌──────────────┐
│    Lexer     │  UTF-8 aware, newline-sensitive, raw spans
└──────┬───────┘
       │ tokens (+ trivia stream for formatter/LSP)
       ▼
┌──────────────┐
│    Parser    │  recoverable recursive-descent + Pratt for expressions
└──────┬───────┘
       │ lossless CST → AST (spans preserved)
       ▼
┌──────────────────────────┐
│   Semantic Analysis      │
│  1. Name resolution      │  (module graph, shadowing, use-before-def rules)
│  2. Type checking        │  (Hindley-Milner-inspired local, annotation-guided global)
│  3. Trait resolution     │  (coherence: one impl per trait+type)
│  4. Exhaustiveness check │  (match, unions)
│  5. Ownership analysis   │  (only if annotations used; see memory_model)
│  6. Effect analysis      │  (async/unsafe/compile-time contamination)
└──────┬───────────────────┘
       │ Typed AST
       ▼
┌──────────────────────────┐
│        Nova IR           │  SSA, explicit alloca/load/store,
│                          │  typed units, monomorphized generics
└──────┬───────────────────┘
       │
       │  Passes: mem2reg · escape analysis + ARC insertion ·
       │  inlining · devirtualization · loop opts · SIMD vectorization ·
       │  dynamic-call inline caching · dead code elim
       │
       ├──► LLVM backend (inkwell) ──► Native (x86-64, ARM64, RISC-V)
       │                       ├ DWARF/PDB debuginfo
       │                       ├ LTO, PGO support
       │                       └ static or dynamic linking
       ├──► VM backend ────► Nova Bytecode ──► Nova VM (REPL, scripting, hot reload)
       ├──► WASM backend ──► wasm32/wasm64 + WASI
       └──► GPU backend ───► CUDA PTX / SPIR-V (Vulkan) / Metal Air
                               (@gpu kernels verified against the GPU-IR subset)
```

Design rule: **each layer only knows its neighbour** through well-defined data types.
Lexer knows Token; parser knows AST; semantics knows TypedAST; IR-gen knows IR.
No layer reaches two steps ahead.

Implementation note (2026-08-23): the native pipeline is written in **Rust**
(README decision log). The Python interpreter in `bootstrap/` is the reference
oracle; its golden AST dumps are the byte-compatibility target for M0.

## 2. Compiler subsystems (Rust workspace)

```text
crates/
├── nova-lexer/       token.rs, lexer.rs, trivia (comments/ws for LSP)
├── nova-parser/      parser.rs, expressions.rs (Pratt), statements.rs, recovery.rs
├── nova-ast/         nodes.rs (arena-allocated), visitor.rs, printer.rs (dump)
├── nova-semantics/
│   ├── name_resolver.rs
│   ├── type_checker.rs          constraint-based inference
│   ├── trait_solver.rs          impl search, coherence check
│   ├── exhaustiveness.rs        match/unions
│   ├── ownership.rs             borrow/move check (opt-in subset)
│   └── effects.rs               async/unsafe/const analysis
├── nova-ir/
│   ├── ir.rs                    module/function/block/instruction
│   ├── builder.rs
│   ├── verify.rs                SSA verifier
│   └── passes/                  see pass list below
├── nova-backend/
│   ├── llvm/         ir→LLVM IR via inkwell, debuginfo, target machines
│   ├── vm/           bytecode emitter, serializer
│   ├── wasm/         direct emitter (no LLVM dependency)
│   └── gpu/          kernel extractor, CUDA/SPIR-V/Metal emit
└── nova-driver/      pipeline orchestration, incremental compilation manager
```

### IR passes (order)

1. `lower-ast-to-ir` (generics monomorphization happens here)
2. `insert-arc` (after escape analysis)
3. `mem2reg`
4. `inline` (cost-model driven, PGO-feedable)
5. `devirtualize` (trait-object calls → direct calls where possible)
6. `simplifycfg`, `licm`, `loop-unroll`, `gvn`
7. `vectorize` (SIMD: SSE/AVX/AVX-512/NEON per target features)
8. `inline-cache-lowering` (dynamic dispatch sites)
9. `tail-call`, `dce`, `stack-size-reduction`

## 3. Nova IR design

- **SSA form** with explicit memory operations (`alloca/load/store/getfield/setfield`).
- **Types in IR**: primitive ints/floats, pointer, struct, array, enum (tagged union),
  closure(env+fn-ptr), dynamic(tagged), opaque.
- **Units & linking**: each module → one IR unit; symbols named
  `module::path::name@mangled-generics` for cross-compilation caching.
- **Ownership as metadata**: ARC insertion is an IR pass, not an AST annotation —
  which is why the `minimal` runtime can skip the pass entirely.
- **Stable text form** (`nova ir dump`) for testing: golden-file tests of the whole pipeline.
- **GPU-IR**: separate strict-subset verifier (no recursion, no heap alloc, limited
  pointer arithmetic) running before the GPU backend.

## 4. Incremental compilation

- The compiler is a **servable library** (the `nova-driver` crate): LSP, build tool and
  CLI share one instance.
- File-granular sparse re-analysis: changed file → re-parse → declaration-tree diff →
  only affected modules re-typechecked (dependency graph with fingerprinting).
- Cache key: `(input-hash, compiler-version, target-triple, flags)` — enables a shared
  compilation cache (`nova build --cache-shared`).

## 5. Backends

| Backend | Status | Use |
|---|---|---|
| LLVM (inkwell) | M1 (first target) | Native desktop/server/embedded |
| VM bytecode | M2 | REPL, scripts, fast dev builds, sandbox |
| WASM | M3 | Browser, edge |
| GPU | M5 | CUDA, Vulkan/SPIR-V, Metal |

Cross-compilation: LLVM target triples + sysroot management in the build tool.
`nova build --target aarch64-linux-gnu --runtime minimal`.

## 6. Runtime

```text
runtime/
├── core/           ARC primitives, type-layout info, panic handler, stack guard
├── gc/             cycle collector (optional, --runtime full)
├── sched/          work-stealing task scheduler, IO reactor (epoll/kqueue/IOCP)
├── async/          futures/state machines, timers, cancellation tokens
├── sync/           Mutex, RwLock, Once, Barrier, Condvar, Atomics
├── io/             file/console/net abstractions over OS APIs
├── dyn/            dynamic-value boxing, method caches
├── reflect/        runtime type metadata (stripped in minimal)
└── platform/       win32/posix abstraction
```

Profiles:

| Profile | Contains | Size approx. |
|---|---|---|
| `minimal` | core + ARC + panic | < 100 KB |
| `core` | + scheduler/async/sync/io/dyn | ~500 KB |
| `full` | + gc/reflect + full stdlib metadata | ~2 MB |

Static linking yields single-file executables; `nova run file.nova` uses the VM/JIT
without a link step.

## 7. VM

- Bytecode: stack-based with register-typed hint fields; compact encoding (LEB128).
- Two tiers: interpreter (baseline) + method JIT with inline caches for `dynamic` calls.
- Hot reload: function versioning, safe points at call boundaries.
- Sandboxing: capability-based (fs/net/process must be granted explicitly).

## 8. Tooling

- **LSP server**: hover, goto-def, find-references, rename, inlay hints (inferred
  types), semantic highlight, auto-import.
- **Formatter**: canonical style, idempotent, uses the lossless token stream.
- **Linter**: rule sets (`naming`, `safety`, `performance`, `idiom`), autofix where possible.
- **Debugger**: DAP server; natively via GDB/LLDB integration, in the VM via own protocol.
- **Doc generator**: `///` doc comments, Markdown output, doctest examples run by
  `nova test --doc`.
- **Fuzzer**: `nova fuzz` property-based testing built in.

## 9. Package ecosystem

- Registry (`registry.nova.dev`): semver, lockfile (`nova.lock`, checksum-signed
  tarballs), platform variants, prebuilt native artifacts.
- `project.nova` is itself valid Nova code (build logic is just code — see module_system).

## 10. Bootstrap cut: REPL (v0.13+, item C09)

`python bootstrap/nova_cli.py repl [--seed N]` starts an interactive session on ONE
persistent `Interp` instance (vars/funcs/things survive across lines).

```text
>>> 1 plus 2
→ 3
>>> x is 5
>>> repeat 2 times
..>      say "hei"
..> done
hei
hei
>>> :ast 1 plus 2
Program (1 statements)
  ExprStmt(line=1)
    ...
>>> :undo
>>> :quit
```

Rules:

1. **Prompts:** `>>> ` for a new sentence, `..> ` while a block is still open.
2. **Multiline via `done`:** if parsing fails with a "missing 'done'" family error,
   collection continues; any other error is reported immediately and the buffer cleared.
   EOF (Ctrl-D / closed stdin) exits cleanly with code 0.
3. **Echo:** an input that IS one expression (ExprStmt) prints its value as `→ value`.
   say/assignments print only their own effect. Lines that fail statement-parse but
   parse as a single expression echo too (e.g. `1 plus 2`).
4. **Meta commands** (lines starting with `:`, case-insensitive):
   - `:ast <line>` — parse without running; prints the AST dump (expression first,
     falling back to statements);
   - `:undo` — restore state from before the last executed chunk (globals/funcs/things
     via deep copy; max 100 steps). Already-printed output and file-I/O cannot be undone;
   - `:quit` (alias `:q`) — exit;
   - `:help` — the list above. Unknown `:command` = friendly error mentioning `:help`.
5. **Errors never kill the session** — same sentence formatting as `run`.
6. Non-interactive use (pipes/tests): same behavior; stdin lines processed until EOF.

## 11. Bootstrap cut: `nova test` runner (v0.20+, item N07)

`nova test [path]` runs every `*.test.nova` discovered recursively under *path* (default: current directory), sorted lexicographically. Each file executes in a fresh VM with its own directory as import base.

```text
PASS/FAIL model:
  a file PASSES when it runs to completion without raising;
  any NovaError fails it (first failure stops that file).
Assertions (stdlib `test`, each raises on violation):
  test.equal(actual, expected)   # semantic equality (nova_eq)
  test.true(condition)           # must be exactly true
  test.fail([message])           # unconditional failure
Output:
  FAIL <relative-path>
        <error message>
  <passed> passed, <failed> failed
Exit codes: 0 = all passed (or no files found, with a note); 1 = any failure.
```
