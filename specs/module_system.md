# Nova Modulsystem & Packages

## 0. Bootstrap-udsnit (v0.12+, item C05) — IMPLEMENTERET i bootstrap v0.12

Én fil = ét modul. Import i Natural-skin:

```text
the tools-module in "tools.nova"     # binder variablen 'tools-module'
say "{tools-module.twice(21)}"       # navnerums-kald MED parenteser
say "{the answer of the tools-module}"   # felt-læsning af modul-variabel
```

Præcise regler:

1. **Import-sætning:** `the NAVN in "STI"` hvor NAVN **skal** ende på `-module`
   (gør sætningen entydig og selvdokumenterende). STI er en streng relativt til
   den importerende fils mappe. Binder NAVN til en modul-værdi.
2. **Separate navnerum:** modulens funktioner, things og top-niveau variabler lever
   i sit eget scope — import ændrer ALDRIG hovedprogrammets globale navne.
3. **Navnerums-kald:** `NAVN.funktion(arg, ...)` — parentes-form kun (ord-formen
   `f with x` gælder ikke for modulkald i bootstrap; almindelige funktioner beholder
   begge former). Kalder man `.funktion(...)` på noget der IKKE er et modul, får man
   en venlig fejl der peger på import-sætningen.
4. **Felt-læsning:** `the NAVN of MODULVARIABEL` / `MODUL.felt` læser modul-global
   eller funktion. Ukendt navn = sætning + did-you-mean + gyldige navne.
5. **Idempotent:** samme fil importeret to gøre køres én gang (cache på absolut sti).
6. **Cirkulær import = fejl** med hele kæden i sætningen
   ("cirkulær import: a.nova → b.nova → a.nova — ...").
7. Et modul **må ikke** indeholde `when the program starts` — det giver en fejl
   ("flyt den til hovedprogrammet"). Manglende fil, lexer-/parserfejl i modulet =
   venlige sætninger med filnavnet i.
8. Moduler kan selv importere andre moduler (kædet op imod egen mappe).

Ikke i bootstrap-udsnittet (kommer senere): `pub`/eksport-grænser, inline `mod {}`,
pakker/project.nova, `as`-alias, selektiv import, ting konstrueret via navnerum.

## 1. Moduler

- Én fil = ét modul. Mappe = undermodul-navnerum (`net/http.nova` → `import net.http`).
- `mod navn { ... }` inline-moduler tilladt.
- Symboler er private som default; `pub` eksporterer.

```text
// src/math_utils.nova
pub fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
fn internal() {}

// andetsteds
from math_utils import lerp
import math_utils as mu
mu.lerp(0, 10, 0.5)
```

Import-regler:

- Stier: `std.*` (stdlib), pakkenavn (dependencies), relative (`./sibling`, `../parent`).
- Cykliske imports tilladt mellem moduler i samme package (lazy resolution); på tværs af packages = fejl.
- `export` i en mod-rodfil definerer den offentlige API-overflade (resten er internt, selv hvis `pub`).

## 2. project.nova

```text
name = "myapp"
version = "1.0.0"
novac = ">= 0.9"
description = "Eksempel-app"
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

[scripts]                   # vilkårlige kommandoer: nova run bench
bench = "nova build --release && ./target/bench"

[profiles.release]
overflow-checks = false
debug-info = "line-tables"
```

`project.nova` er selv gyldig Nova-kode (TOML-agtig syntax er et subset) — build-logik kan programmeres direkte:

```text
[prebuild]
fn prebuild(cfg) {
    cfg.generate("version.nova", 'pub const VERSION = "{cfg.version}"')
}
```

### Capability-tilladelser — BESLUTTET

Scripts og pakker erklærer deres behov; runtime håndhæver dem (se docs/EXTENSIONS.md E2):

```text
[permissions]               # default: ALT false for dependencies
read  = ["data/*"]          # glob-scoped fillæsning
write = false
network = ["api.example.com"]
spawn  = false
ffi    = []                 # liste af native-biblioteker
```

- `nova install` viser tilladelses-dialog ved pakker med nye rettigheder.
- Overtrædelse runtime = panik `PermissionError`; overtrædelse statisk detekterbar = compile-fejl.
- `--allow-all` deaktiverer (kun til udviklerens eget program, aldrig dependencies).

## 3. Package manager (`nova pm`)

```text
nova init [template]        # bin | lib | wasm | gpu
nova add <pkg>[@version]    # + --dev, --feature
nova remove <pkg>
nova update [<pkg>]         # semver-respekt; --major tillader breaking
nova publish                # checksum-signeret tarball til registry
nova install .              # installer binary globalt
```

- Lockfile `nova.lock` (eksakt, checksummet, committes).
- Registry: `registry.nova.dev` — semver, yank, trusted publishers, platform-varianter.
- Native artifacts: pakker kan shippe prebuildede `.a/.so/.dll` per target + source-fallback.
- Workspace-support: monorepo med flere packages (`members = ["apps/*", "libs/*"]`).

## 4. Build

```text
nova build [--release|--debug] [--target triple] [--runtime minimal|core|full]
nova run [fil|script-navn]  # dev-build + kør
nova test [--doc] [--bench]
nova fmt / nova lint / nova doc / nova repl
nova check                  # typetjek uden codegen (hurtig CI-feedback)
```

- Inkrementelt, content-hash-cachet, paralliseret over kerner.
- Cross-compiling: targets defineret i `[targets.table]` eller CLI; sysroots hentes automatisk hvor muligt.
- Output: `target/debug|release/<triple>/...`.

## 5. Versionspolitik og stabilitet

- Semver. Edition-nøgle (`edition = "2026"`) giver sprog-evolution uden at knække gammel kode.
- Stdlib følger sproget; deprecated API'er beholder mindst 2 minor-versioner.
