# Nova Roadmap

Principle: every milestone ships something real. The native pipeline is written in
**Rust** (owner decision 2026-08-23); the Python bootstrap is the differential oracle.

## Current state (2026-08-24)

| Track | Version | What exists |
|---|---|---|
| Python bootstrap (oracle) | v0.15.1-bootstrap | Natural + shorthand skins, Optional/`?`, modules, stdlib v0, REPL, memory semantics; 236/236 end-to-end tests |
| Native (Rust) — **Phase 0.2** | v0.16.0-native → ships **v0.20.0** | Lexer/parser at golden parity, bytecode compiler + stack VM, runtime wave (check/try/contracts/things/track-undo/phrase builtins/`?`/interpolation), modules + stdlib v0, `nova test`, field pack, history & Flow v0 |

Live board: [ITERATION_PLAN §6 Phase 0.2](ITERATION_PLAN.md).

## Phase 0.2 — native release (v0.20.0)

Bytecode compiler + stack VM behind a swappable-backend boundary (LLVM/JIT can slot in
later). Dynamic core + opt-in annotations; arbitrary-precision integers through 0.2;
stdlib order cli/csv/datetime/regex. Uniqueness bets: history engine + Flow<T>.
Tooling floor: `nova test` + LSP.

Done: N00 grammar freeze · N01 Rust lexer · N02 parser (golden byte-equal) ·
N03 compiler+VM core · N04 full runtime wave incl. modules+stdlib · N05 differential
harness 29/29 · **N07 `nova test`** · **N06 cli/csv/datetime/regex** ·
**N08a history freeze+queries** · **N08b Flow freeze+list ops**.
Deferred by owner decision: **N09 LSP → v0.21**.

## M-milestones (updated)

- **M0-M2 (pulled early, mostly done via Phase 0.2):** lexer/parser/AST ✅ · name
  resolution + Optional ✅ · try/catch + contracts ✅ · VM backend ✅ · stdlib core ✅ ·
  test runner ✅ · LSP v1 → v0.21
- **M1 remainder:** Nova IR (SSA) + verifier; LLVM backend via inkwell; ARC pass +
  escape analysis; benchmarks vs C++/Python on fib/matmul/sort
- **M3+:** package manager + registry + lockfile; incremental compilation;
  regex/formats/databases beyond the field pack; debugger (DAP); WASM backend
- **M4+:** dynamic specialization, reflection, crypto/TLS/compression, cross-compile matrix
- **M5+:** nova-gui, GPU backend, nova-array/plot, time statements at scale
- **M6+:** self-hosting tooling (nova fmt in Nova), plugin API, edition-2026 spec freeze

See [EXTENSIONS.md](EXTENSIONS.md) for the 16 extension proposals and
[specs/unique_features.md](../specs/unique_features.md) for the 13 unique features.
History engine: [specs/history_engine.md](../specs/history_engine.md) (frozen).
Flow: [specs/flow.md](../specs/flow.md) (frozen).
