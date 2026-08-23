# Nova Roadmap

Principle: every milestone must run real programs and be useful on its own. No
milestone blocks on "all features".

**Native implementation language: Rust** (owner decision 2026-08-23 — replaces the
original C++ assumption; recorded in README decision log).

## M0 — Bootstrap (week 1-4)
- Lexer + lossless tokens/trivia, Pratt parser, AST + printer (**Rust** workspace)
- Golden tests from day 1 (`dump` format) — must match the Python bootstrap's
  `tests/golden/*.ast.txt` byte-for-byte
- CLI skeleton: `nova parse`, `nova ast`

## M1 — Can compute (month 1-3)
- Name resolution, type inference (HM-local), unions, Optional/Result
- Nova IR + verifier; LLVM backend via **`inkwell`** (x86-64 + ARM64)
- ARC pass + escape analysis
- stdlib-core: ints/floats/String/Array/Map/Set/tuple/range/iterator/comprehensions
- `nova build` / `nova run` native
- **Goal:** benchmarks vs C++/Python on fib/matmul/sort

## M2 — Pleasant to use (month 3-6)
- Match exhaustiveness, traits+generics+monomorphization, lambdas/closures
- Complete error handling (`?`, flow typing), defer/use, @test-runner
- VM backend (bytecode + interpreter) → REPL, scripting, shebang
- async/await + scheduler + channels/select; the `parallel` block
- LSP v1 (hover, goto-def, diagnostics, inlay hints); formatter; linter v1
- stdlib: fs/process/env/io/time/log/cli/iter/func/random/stats
- FFI: import c "header.h" (libclang-style importer)

## M3 — The ecosystem opens (month 6-10)
- Package manager + registry + lockfile + workspaces
- Incremental compilation + shared cache
- std.regex, std.formats (csv/toml/yaml/xml), std.database (sqlite),
  std.net (tcp/udp/http client+server/websocket), std.serialization
- WASM backend (wasi)
- Debugger integration (DAP over GDB/LLDB + VM protocol)

## M4 — Enterprise features (month 10-14)
- `dynamic` complete with inline caches (+ first specialization pass)
- Reflection runtime (full profile) + derive macros + comptime evaluation
- std.crypto, TLS, compression/archives, mmap
- GC cycle collector (full profile), stable runtime profiles (minimal/core/full)
- Cross-compilation matrix CI

## M5 — Graphics/GUI/science (month 14-18)
- nova-gui (declarative widgets, async event loop)
- GPU backend: @gpu kernels → CUDA + SPIR-V/Vulkan + Metal
- nova-array (ndarray + BLAS/LAPACK + fft), SIMD vectorizer stable
- nova-plot

## M6 — Mature platform (18+ months)
- nova-ml (autograd), ORM layer, i18n
- Java/JVM bridge (JNI interface), Python interop beyond embedding
- PGO/LTO workflows, compiler plugin API (macro SDK)
- Spec freeze: edition 2026, formalized semver for the language itself

See [EXTENSIONS.md](EXTENSIONS.md) for 16 worked-through extension proposals (units,
signals, actors, contracts, capability sandbox, time-travel debugger, notebook mode,
teaching pack) with per-feature milestone placement.

See also [../specs/unique_features.md](../specs/unique_features.md) for the 13 DECIDED
unique features (Flow, Table, undo/history, taint, state machines, exact blocks,
sim-test, @incremental, nova why, grammar literals, pure-Nova stack) with milestones.
