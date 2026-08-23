# Nova Roadmap

Princip: hver milepæl skal kunne køre ægte programmer og være nyttig alene. Ingen milepæl blokerer på "alle features".

## M0 — Bootstrap (uge 1-4)
- Lexer + lossless tokens/trivia, Pratt-parser, AST + printer
- Golden tests fra dag 1 (`dump`-format)
- CLI-skelet: `nova parse`, `nova ast`

## M1 — Kan beregne (måned 1-3)
- Name resolution, type-inference (HM-lokal), unions, Optional/Result
- Nova IR + verifier, LLVM-backend (x86-64 + ARM64)
- ARC-pass + escape analysis
- stdlib-core: ints/floats/String/Array/Map/Set/tuple/range/iterator/comprehensions
- `nova build` / `nova run` native
- **Mål:** benchmarks vs C++/Python på fib/matmul/sort

## M2 — Behagelig at bruge (måned 3-6)
- match ekshaustivitet, traits+generics+monomorphisering, lambdas/closures
- Fejlhåndtering komplet (? , flow-typing), defer/use, @test-runner
- VM-backend (bytecode + interpreter) → REPL, scripting, shebang
- async/await + scheduler + channels/select; parallel-blokken
- LSP v1 (hover, goto-def, diagnostics, inlay hints); formatter; linter v1
- stdlib: fs/process/env/io/time/log/cli/iter/func/random/stats
- FFI: import c "header.h" (libclang-agtig importer)

## M3 — Økosystemet åbner (måned 6-10)
- Package manager + registry + lockfile + workspaces
- Incremental compilation + shared cache
- std.regex, std.formats (csv/toml/yaml/xml), std.database (sqlite),
  std.net (tcp/udp/http client+server/websocket), std.serialization
- WASM-backend (wasi)
- Debugger-integration (DAP over GDB/LLDB + VM-protokol)

## M4 — Enterprise-funktioner (måned 10-14)
- dynamic komplet med inline caches (+ første specialiseringspass)
- Reflection-runtime (full-profil) + derive-makroer + comptime-evaluering
- std.crypto, TLS, compression/archives, mmap
- GC-cycle collector (full-profil), runtime-profiler stabile (minimal/core/full)
- Cross-compilation-matrix CI

## M5 — Grafik/GUI/videnskab (måned 14-18)
- nova-gui (deklarativ widgets, async event-loop)
- GPU-backend: @gpu kernels → CUDA + SPIR-V/Vulkan + Metal
- nova-array (ndarray + BLAS/LAPACK + fft), SIMD-vectorizer stabil
- nova-plot

## M6 — Moden platform (18+ måneder)
- nova-ml (autograd), ORM-lag, i18n
- Java/JVM-bridge (JNI-grænseflade), Python-interop ud over embedding
- PGO/LTO-workflows, plugin-API til compileren (makro-SDK)
- Spec-frysning: edition 2026, formaliseret semver for selve sproget

Se [EXTENSIONS.md](EXTENSIONS.md) for 16 gennemarbejdede udvidelsesforslag (units, signals,
actors, contracts, capability-sandbox, time-travel debugger, notebook-mode, undervisningspakke)
med milepæls-placering pr. feature.

Se desuden [../specs/unique_features.md](../specs/unique_features.md) for de 13 BESLUTTEDE
unikke features (Flow, Table, undo/historik, taint, tilstandsmaskiner, exact-blokke,
sim-test, @incremental, nova why, grammatik-literals, pure-Nova stacken) med milepæle.

## Løbende kvalitets-mål
- Hvert lag: unit tests + golden tests; pipeline: end-to-end snapshot-tests
- Performance-regression-suite pr. commit (bench-statistik med varians)
- Fuzzing af lexer/parser (cargo-fuzz-agtig), differential-test native vs VM
- Fejlmeddelelses-kvalitet som feature: hver diagnostic har kode + fix-hint
