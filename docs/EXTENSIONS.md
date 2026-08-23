# Nova Extensions — DECIDED extensions v1

Status: **All 16 features approved and integrated into the kernel specifications.**

| Feature | Home in specs |
|---|---|
| A1 Refinement types | type_system.md §11 |
| A2 Units | type_system.md §12 + standard_library.md §21 |
| A3 Verified format strings | language_reference.md §7 (strings) — compiler pass |
| B1 Pipelines / `then` | language_reference.md §20 |
| C1 Signals | language_reference.md §21 |
| D1 Actors | concurrency.md §7 |
| E1 Contracts | language_reference.md §23 |
| E2 Capability permissions | module_system.md (project.nova `[permissions]`) |
| E3 Reproducible builds | module_system.md §4 (build flag) + registry |
| F1 Time-travel debugger | docs/ARCHITECTURE.md §8 tooling |
| F2 Notebook/literate | ARCHITECTURE.md §8 (`nova notebook`, `.nova.md`) |
| F3 `nova explain` | diagnostics spec (error ID + offline knowledge base) |
| F4 API diff | `nova api-diff` CLI |
| F5 Teaching pack + Blocks | `nova trace`, block editor → natural export |
| G1 Embedding API | runtime/minimal profile + libnova_vm.h |
| G2 Native hot reload | dev builds' dynamic libraries |

Below: original motivations and examples (kept as design rationale).

---

## Theme A — Types that describe themselves and catch more errors

### A1. Refinement types (conditional types)

Types with built-in rules — checked at the boundary, optimized away inside.

```text
# Natural
an age is a whole number from 0 to 130
a positive is a number greater than 0

to buy-ticket with age: age
    # the compiler GUARANTEES here that age is valid
done
```

```text
# Compact
type Age = i32 where self >= 0 && self <= 130
type NonEmpty<T> = Array<T> where self.len() > 0

fn buy_ticket(age: Age) { ... }
buy_ticket(user.age)              # runtime check at the call boundary
buy_ticket(-5)                    # COMPILE error if constant, otherwise runtime check
```

Implementation: subtype of base type + predicate; SMT-light verification of constant arguments, otherwise an automatic boundary check. Zero cost inside the function when flow analysis can prove the predicate.
**Why:** removes an entire class of validation boilerplate; a perfect match for natural-syntax readability. Risk: low-medium. **Milestone: M4.**

### A2. Units and dimensions (units of measurement)

```text
# Natural
the distance is 100 meters
the time was 9.58 seconds
the speed is the distance divided by the time       # m/s inferred
say "{the speed in kilometers per hour}"

the distance plus the time                          # COMPILE ERROR: meter + second
```

```text
# Compact
let d = 100.m
let t = 9.58.s
let v = d / t            # Unit<Length/Time> — dimensional analysis in the type system
v.to::<km/h>()
```

Implementation: generic `Unit<L,M,T,...>` with integer exponents; monomorphizes to raw floats — **zero runtime cost**. SI units + prefixes in stdlib; currency based on `Decimal`.
**Why:** Nova wants to replace Python in scientific computing — NASA's Mars Climate Orbiter loss (~$327M) was a unit error. Risk: medium (type-system work). **Milestone: M5 (with nova-array).**

### A3. Compile-time verified format strings

`"{price:.2} kr"` is verified against argument types at compile time: wrong specifier (`{name:.2}` on String) or a missing variable = compile error, not a runtime crash. Applies to `say`, `format`, logging. Regex literals are already compile-checked — this completes the pattern.
Risk: low. **Milestone: M2.**

---

## Theme B — New expression forms

### B1. Pipelines (`|>` and `then` chains)

```text
# Compact
contents |> split_lines |> filter(l => l.len() > 0) |> sorted() |> join("\n") |> print
```

```text
# Natural — speaks exactly like you think:
take the file contents
    then split it by lines
    then keep the ones that are not empty
    then sort them
    then say the result
done
```

Desugars to ordinary method calls (zero-cost). `then` is a reserved word only in pipeline context. **Milestone: M2.**

---

## Theme C — Reactivity (GUI/games without boilerplate)

### C1. Signals — automatic derived values

```text
# Natural
the score is a signal starting at 0
the rank is when the score changes: "Level {floor(score / 100)}"

when the score changes
    say "{score} → {rank}"
done

add 50 to the score          # → automatically: "50 → Level 0"
add 60 to the score          # → "110 → Level 1" (rank updated itself)
```

```text
# Compact
let score = signal(0)
let rank = computed(() => "Level {score.value / 100}")
effect { print("{score.value} → {rank.value}") }
score.value += 60
```

Pull-based, glitch-free (topological invalidation) — same model as SolidJS/SwiftUI `@Observable`. Becomes the foundation under nova-gui: `the button text binds to the label` — UI that simply *is* its state. **Why:** the GUI push is M5; signals must be in the kernel first, not bolted on retroactively. Risk: medium. **Milestone: M4 (kernel) → M5 (GUI integration).**

---

## Theme D — Concurrency extensions

### D1. Actors — isolated state with a message protocol

```text
a BankAccount is an actor keeping
    a balance of 0

    on deposit with amount
        add amount to my balance
    on withdraw with amount
        if my balance is at least amount then
            take amount from my balance
            reply with "ok"
        otherwise
            reply with "not enough money"
    done
done

account is a new BankAccount
send account "deposit" with 100
answer is ask account to "withdraw" with 150     # request/response, waits for the reply
```

One message at a time per actor → no locks, no data races possible. Implemented on top of existing tasks + channels (no new runtime). Good fit for game entities, servers and later distributed systems. **Milestone: M4.**

---

## Theme E — Robustness and security

### E1. Contracts (requirements and promises)

```text
to withdraw with amount
    requires amount is greater than 0
    requires amount is at most my balance
    ensures my balance is at least 0

    take amount from my balance
done
```

Compact: `@requires(x > 0) @ensures(result >= 0)`. Checked in debug/tests; stripped in release (or kept via profile). The same mechanism drives: the fuzzing generators (the contracts ARE the test-input rules), documentation (shown in hover/doc) and future lightweight formal verification. **Milestone: M3.**

### E2. Capability permissions for scripts and packages

```text
# project.nova
[permissions]
read  = ["data/*"]
write = false
network = ["api.example.com"]
spawn  = false
```

- Scripts/packages must **declare** what they need (like app permissions on a phone).
- The runtime enforces it; `nova install` shows a permission dialog.
- Malicious supply-chain attacks are limited from "full machine" to "declared scope".
Builds on the existing VM sandbox; native builds get it via a runtime hook on the fs/net/process APIs. **Milestone: M4.**

### E3. Reproducible builds + signed provenance

- `nova build --reproducible`: byte-identical output from the same source + compiler version (frozen stdlib, deterministic codegen).
- The registry stores a build manifest + signature; `nova verify <pkg>` checks the chain.
**Why:** ecosystem trust (the XZ-utils lesson). **Milestone: M4-M5.**

---

## Theme F — Tooling no competitor has

### F1. Time-travel debugger

The VM records execution (variable history in ring buffers). `nova replay` lets you **rewind**:

```text
BREAK at tour.nova:42 (iteration 847 of 1000)
  guess = 62   secret = 62   tries = 7
[← back] [forward →] [where did 'tries' come from?] [watch: secret]
```

"Where did this variable get its value?" = reverse dataflow search. The native build's instrumentation mode provides the same (slower). **Why:** beginner-friendly AND a pro tool; RR/time-travel exist in the C++ world but are rare and hard — here it is standard. Risk: high complexity, but the VM already owns the entire execution. **Milestone: M5-M6.**

### F2. Notebook + literate mode

- `nova notebook` → local web kernel (Jupyter protocol): cells, inline plots (nova-plot), markdown between code.
- `.nova.md` files: ```nova fences executed by `nova test --doc` — documentation that NEVER goes stale because it is tested.
**Milestone: M4 (kernel), M6 (plot integration).**

### F3. `nova explain` — error messages with a teacher

```text
error[E2031]: you compared a text with a number
  --> guessing.nova:12:24
   |
12 |     if answer is less than secret then
   |         ------          ------
   |         text ('"75"')   number (i32)
   |
help: convert first:  set guess to the number value of answer
note: why: texts compare letter by letter ("10" < "9"!)
```

Every diagnostic has a stable ID + an offline knowledge base (`nova explain E2031`). No cloud dependency. **Milestone: ongoing from M1.**

### F4. API diff and semver police

`nova api-diff v1.2.0..HEAD` → list of added/removed/changed public symbols + semver verdict: *"breaking: parameter `timeout` had its default value removed → requires a major bump"*. CI gate for packages. **Milestone: M5.**

### F5. The teaching pack

- `nova trace file.nova` — Python-Tutor-like line-by-line visualization of all variables (terminal + web).
- **Nova Blocks**: a Scratch-like block editor that **exports to Nova Natural text** (one way — no lock-in). The Natural syntax is designed for exactly this: every block = one sentence.
**Milestone: M6+.**

---

## Theme G — Platform reach

### G1. Embedding API (Nova's Lua niche)

`libnova_vm.h` / `libnova.so`: host applications (C/C++/C#/Rust/game engines) embed the Nova VM as a scripting layer:

```c
nova_vm* vm = nova_new(&(nova_opts){ .permissions = NOVA_CAP_NONE });
nova_reg(vm, "move_player", host_move_player);
nova_run(vm, nova_readfile("level1.nova"));
```

The minimal profile (< 100 KB, static, no GC) is already designed for exactly this. Mod-scripting, plugin systems, configuration-as-code. **Milestone: M4.**

### G2. Hot reload for native dev builds too

VM mode has hot reload from day 1. Dev-native mode builds to dynamic libraries and swaps at safe points (the game-engine pattern). Not guaranteed for all programs — the compiler reports exactly when a reload requires a restart. **Milestone: M6.**

---

## Prioritized overview

| # | Feature | Value | Cost | Milestone |
|---|---|---|---|---|
| 1 | A3 Format strings verified at compile time | High | Low | M2 |
| 2 | B1 Pipelines / `then` | High | Low | M2 |
| 3 | F3 `nova explain` | High | Low | M1+ |
| 4 | E1 Contracts | High | Medium | M3 |
| 5 | C1 Signals | Very high (foundation of the GUI push) | Medium | M4 |
| 6 | D1 Actors | High | Medium | M4 |
| 7 | E2 Capabilities/sandbox | Very high (security) | Medium | M4 |
| 8 | G1 Embedding API | High (new audience) | Medium | M4 |
| 9 | F2 Notebook/literate | High | Medium | M4 |
| 10 | A1 Refinement types | Medium-high | Medium | M4 |
| 11 | E3 Reproducible builds | Medium-high | Medium | M4-M5 |
| 12 | A2 Units | Very high for science | High | M5 |
| 13 | F1 Time-travel debug | Very high (differentiator) | High | M5-M6 |
| 14 | F4 API diff | Medium | Low-medium | M5 |
| 15 | F5 Teaching pack + Blocks | Strategically high | High | M6+ |
| 16 | G2 Native hot reload | Medium | High | M6 |
