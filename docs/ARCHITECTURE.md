# Nova Arkitektur

## 1. Pipeline-oversigt

```text
Source (.nova)
    │
    ▼
┌──────────────┐
│    Lexer     │  UTF-8 aware, newline-sensitive, raw spans
└──────┬───────┘
       │ tokens (+ trivia stream til formatter/LSP)
       ▼
┌──────────────┐
│    Parser    │  recoverable recursive-descent + Pratt for expressions
└──────┬───────┘
       │ lossless CST → AST (spans bevaret)
       ▼
┌──────────────────────────┐
│   Semantic Analysis      │
│  1. Name resolution      │  (modul-graf, shadowing, use-before-def regler)
│  2. Type checking        │  (Hindley-Milner-inspireret lokalt, annotation-guidet globalt)
│  3. Trait resolution     │  (coherence: én impl per trait+type)
│  4. Exhaustiveness check │  (match, unions)
│  5. Ownership analysis   │  (kun hvis annotations bruges; se memory_model)
│  6. Effect analysis      │  (async/unsafe/compile-time kontaminering)
└──────┬───────────────────┘
       │ Typed AST
       ▼
┌──────────────────────────┐
│        Nova IR           │  SSA, explicit alloca/load/store,
│                          │  typed units, monomorphiserede generics
└──────┬───────────────────┘
       │
       │  Passes: mem2reg · escape analysis + ARC insertion ·
       │  inlining · devirtualization · loop opts · SIMD vectorization ·
       │  dynamic-call inline caching · dead code elim
       │
       ├──► LLVM backend ──► Native (x86-64, ARM64, RISC-V)
       │                       ├ DWARF/PDB debuginfo
       │                       ├ LTO, PGO-understøttelse
       │                       └ statisk eller dynamisk linking
       ├──► VM backend ────► Nova Bytecode ──► Nova VM (REPL, scripting, hot reload)
       ├──► WASM backend ──► wasm32/wasm64 + WASI
       └──► GPU backend ───► CUDA PTX / SPIR-V (Vulkan) / Metal Air
                               (@gpu-kernels verificeret mod GPU-IR subset)
```

Designregel: **hvert lag kender kun sin nabo** via veldefinerede datatyper. Lexer kender Token; parser kender AST; semantic kender TypedAST; IR-gen kender IR. Ingen lag inkluderer to skridt frem.

## 2. Compiler-subsystemer

```text
compiler/
├── lexer/          token.cpp/h, lexer.cpp, trivia (kommentarer/ws til LSP)
├── parser/         parser.cpp, expressions.cpp (Pratt), statements.cpp, recovery.cpp
├── ast/            nodes.h (arena-allokeret), visitor.h, printer.h (dump)
├── semantics/
│   ├── name_resolver.cpp
│   ├── type_checker.cpp        constraint-baseret inference
│   ├── trait_solver.cpp        impl-søgning, coherence-check
│   ├── exhaustiveness.cpp      match/unions
│   ├── ownership.cpp           borrow/move-check (opt-in delmængde)
│   └── effects.cpp             async/unsafe/const analyse
├── ir/
│   ├── ir.h                    module/function/block/instruction
│   ├── builder.cpp
│   ├── verify.cpp              SSA-verifikator
│   └── passes/                 se pass-liste nedenfor
├── backend/
│   ├── llvm/       ir→LLVM IR, debuginfo, target-machines
│   ├── vm/         bytecode-emitter, serializer
│   ├── wasm/       direct emitter (ingen LLVM-afhængighed)
│   └── gpu/        kernel-extractor, CUDA/SPIR-V/Metal emit
└── driver.cpp      pipeline-orkestrering, incremental compilation manager
```

### IR-passes (rækkefølge)

1. `lower-ast-to-ir` (monomorphisering af generics her)
2. `insert-arc` (efter escape analysis)
3. `mem2reg`
4. `inline` (cost-model drevet, PGO-feedable)
5. `devirtualize` (trait-object kald → direkt kald hvor muligt)
6. `simplifycfg`, `licm`, `loop-unroll`, `gvn`
7. `vectorize` (SIMD: SSE/AVX/AVX-512/NEON efter target features)
8. `inline-cache-lowering` (dynamic dispatch sites)
9. `tail-call`, `dce`, `stack-size-reduction`

## 3. Nova IR-design

- **SSA-form** med eksplicitte hukommelsesoperationer (`alloca/load/store/getfield/setfield`).
- **Typer i IR**: primitive ints/floats, pointer, struct, array, enum(tagged union), closure(env+fn-ptr), dynamic(tagged), opaque.
- **Units & linking**: hvert modul → én IR-unit; symbols navngives `module::path::name@mangled-generics` for cross-compilation cache.
- **Ownership som metadata**: ARC-insertion er et pass over IR, ikke en AST-annotering — derfor kan `minimal`-runtime droppe passet helt.
- **Stabil tekstform** (`nova ir dump`) til test: golden-file tests af hele pipelinen.
- **GPU-IR**: separat strict subset-verifikator (ingen rekursion, ingen heap-alloc, begrænset pointer-arithmetic) der kører før GPU-backend.

## 4. Incremental compilation

- Compileren er et **serverbart bibliotek** (`libnovac`): LSP, build-tool og CLI deler samme instans.
- Fil-granuleret sparsom genanalyse: ændret fil → re-parse → diff af deklarerings-træ → kun berørte moduler re-typetjekkes (afhængighedsgraf med fingerprinting).
- Cache-nøgle: `(input-hash, compiler-version, target-triple, flags)` — muliggør shared compilation cache (`nova build --cache-shared`).

## 5. Backends

| Backend | Status | Anvendelse |
|---|---|---|
| LLVM | M1 (første mål) | Native desktop/server/embedded |
| VM bytecode | M2 | REPL, scripts, hurtige dev-builds, sandbox |
| WASM | M3 | Browser, edge |
| GPU | M5 | CUDA, Vulkan/SPIR-V, Metal |

Cross-compilation: LLVM-target triples + sysroot-management i build-tool. `nova build --target aarch64-linux-gnu --runtime minimal`.

## 6. Runtime

```text
runtime/
├── core/           ARC-primitiver, type-layout-info, panik-handler, stack-guard
├── gc/             cycle collector (valgfri, --runtime full)
├── sched/          work-stealing task scheduler, IO-reactor (epoll/kqueue/IOCP)
├── async/          futures/state machines, timers, cancellation-tokens
├── sync/           Mutex, RwLock, Once, Barrier, Condvar, Atomics
├── io/             file/console/net abstraktioner over OS-APIs
├── dyn/            dynamic-value boxe, method caches
├── reflect/        runtime type-metadata (strippes i minimal)
└── platform/       win32/posix abstraction
```

Profiler:

| Profil | Indeholder | Størrelse ca. |
|---|---|---|
| `minimal` | core + ARC + panik | < 100 KB |
| `core` | + scheduler/async/sync/io/dyn | ~500 KB |
| `full` | + gc/reflect + fuld stdlib-metadata | ~2 MB |

Static-linking giver single-file executables; `nova run fil.nova` bruger VM/JIT uden link-step.

## 7. VM

- Bytecode: stackbaseret med register-typed hint-felter; kompakt encoding (LEB128).
- To tiers: interpreter (baseline) + method-JIT med inline caches for `dynamic`-kald.
- Hot reload: funktion-versionering, safe-points ved call-grænser.
- Sandboxing: capability-baseret (fs/net/process skal eksplisit gives).

## 8. Tooling

- **LSP-server**: hover, goto-def, find-references, rename, inlay hints (inferrede typer), semantic highlight, auto-import.
- **Formatter**: kanonisk stil, idempotent, bruger lossless token-stream (bevarer ikke brugerens whitespace).
- **Linter**: regel-sæt (`naming`, `safety`, `performance`, `idiom`), autofix hvor muligt.
- **Debugger**: DAP-server; nativt via GDB/LLDB-integration, i VM via egen protokol.
- **Doc-generator**: `///`-dokumentkommentarer, Markdown-output, doctest-eksempler køres af `nova test --doc`.
- **Fuzzer**: `nova fuzz` property-based testing indbygget.

## 9. Package-økosystem

- Registry (`nova.dev`): semver, lockfile (`nova.lock`, checksum-signerede tarballs), platform-varianter, prebuilt-native-artifacts.
- `project.nova` er selv et Nova-program (build-scripts er bare kode, se module_system).
