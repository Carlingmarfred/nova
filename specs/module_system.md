# Nova Modulsystem & Packages

## 0. Bootstrap cut (v0.12+, item C05) — IMPLEMENTED in bootstrap v0.12

Én fil = ét modul. Import i Natural-skin:

```text
the tools-module in "tools.nova"     # binder variablen 'tools-module'
say "{tools-module.twice(21)}"       # navnerums-kald MED parenteser
say "{the answer of the tools-module}"   # field read of a module variable
```

Precise rules:

1. **Import statement:** `the NAME in "PATH"` where NAME **must** end in `-module`
   (making the sentence unambiguous and self-documenting). PATH is a string relative to
   the importing file's directory. Binds NAME to a module value.
2. **Separate navnerum:** modulens funktioner, things og top-niveau variabler lever
   in its own scope — an import NEVER changes the main program's global names.
3. **Navnerums-kald:** `NAVN.funktion(arg, ...)` — parentes-form kun (ord-formen
   `f with x` does not apply to module calls in the bootstrap; ordinary functions keep
   both forms). Calling `.function(...)` on something that is NOT a module gives
   a friendly error pointing at the import statement.
4. **Field reads:** `the NAME of MODULEVAR` / `MODULE.field` read a module global
   or function. Unknown name = sentence + did-you-mean + valid names.
5. **Idempotent:** importing the same file twice runs it once (cache on absolute path).
6. **Circular import = error** with the whole chain in the message
   ("circular import: a.nova → b.nova → a.nova → ...").
7. A module **must not** contain `when the program starts` — it gives an error
   ("flyt den til hovedprogrammet"). Manglende fil, lexer-/parserfejl i modulet =
   friendly sentences with the filename in them.
8. Modules can import other modules themselves (chained relative to their own directory).

Not in the bootstrap cut (arrives later): `pub`/export boundaries, inline `mod {}`,
pakker/project.nova, `as`-alias, selektiv import, ting konstrueret via navnerum.

## 1. Moduler

- One file = one module. A directory = a submodule namespace (`net/http.nova` → `import net.http`).
- `mod navn { ... }` inline-moduler tilladt.
- Symbols are private by default; `pub` exports.

```text
// src/math_utils.nova
pub fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
fn internal() {}

// andetsteds
from math_utils import lerp
import math_utils as mu
mu.lerp(0, 10, 0.5)
```

Import rules:

- Paths: `std.*` (stdlib), package name (dependencies), relative (`./sibling`, `../parent`).
- Cyclic imports allowed between modules in the same package (lazy resolution); across packages = error.
- `export` in a module root file defines the public API surface (everything else internal, even if `pub`).

## 2. project.nova

```text
name = "myapp"
version = "1.0.0"
novac = ">= 0.9"
description = "Example app"
license = "MIT"

[targets]
main = "src/main.nova"

[dependencies]
graphics = "2.0"
networking = { version = "1.4", features = ["tls", "http2"] }
native-blas = { version = "0.3", platform = ["windows-x64", "linux-x64"] }
local-tool = { path = "../tool" }
dev-dependencies = { testkit = "1.1" }

[features]
default = ["gui"]
gui = ["graphics/full"]

[build]
runtime = "core"           # minimal | core | full
optimization = "release"
lto = true

[scripts]                   # arbitrary commands: nova run bench
bench = "nova build --release && ./target/bench"

[profiles.release]
overflow-checks = false
debug-info = "line-tables"
```

`project.nova` is itself valid Nova code (the TOML-like syntax is a subset) — build logic can be programmed directly:

```text
[prebuild]
fn prebuild(cfg) {
    cfg.generate("version.nova", 'pub const VERSION = "{cfg.version}"')
}
```

### Capability-tilladelser — BESLUTTET

Scripts and packages declare their needs; the runtime enforces them (see docs/EXTENSIONS.md E2):

```text
[permissions]               # default: EVERYTHING false for dependencies
read  = ["data/*"]          # glob-scoped file reading
write = false
network = ["api.example.com"]
spawn  = false
ffi    = []                 # liste af native-biblioteker
```

- `nova install` shows a permission dialog for packages requesting new rights.
- Runtime violation = panic `PermissionError`; statically detectable violation = compile error.
- `--allow-all` disables it (only for the developer's own program, never dependencies).

## 3. Package manager (`nova pm`)

```text
nova init [template]        # bin | lib | wasm | gpu
nova add <pkg>[@version]    # + --dev, --feature
nova remove <pkg>
nova update [<pkg>]         # semver-respecting; --major allows breaking
nova publish                # checksum-signed tarball to the registry
nova install .              # install the binary globally
```

- Lockfile `nova.lock` (exact, checksummed, committed).
- Registry: `registry.nova.dev` — semver, yank, trusted publishers, platform variants.
- Native artifacts: packages can ship prebuilt `.a/.so/.dll` per target + source fallback.
- Workspace support: monorepo with several packages (`members = ["apps/*", "libs/*"]`).

## 4. Build

```text
nova build [--release|--debug] [--target triple] [--runtime minimal|core|full]
nova run [file|script-name] # dev-build + run
nova test [--doc] [--bench]
nova fmt / nova lint / nova doc / nova repl
nova check                  # typecheck without codegen (fast CI feedback)
```

- Incremental, content-hash cached, parallelized across cores.
- Cross-compiling: targets defined in `[targets.table]` or CLI; sysroots fetched automatically where possible.
- Output: `target/debug|release/<triple>/...`.

## 5. Versioning policy and stability

- Semver. The edition key (`edition = "2026"`) enables language evolution without breaking old code.
- Stdlib follows the language; deprecated APIs kept for at least 2 minor versions.
