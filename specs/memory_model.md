# Nova Memory Model

## 0. Bootstrap cut (v0.14+, item C13)

Native ARC/escape-analysis is an IR pass (E05, gate G3). The bootstrap pins NOW the
semantics that ARC must later preserve — and provides one explicit copy phrase.

### 0.1 Value vs reference semantics (holds ALWAYS, native too)

| Value | Semantics | Assignment/call |
|---|---|---|
| numbers, text, bool | **value** | copied (independent) |
| `nothing` | value (one instance) | — |
| lists `[...]` | **reference** | shared (alias) |
| thing instances | **reference** | shared (alias) |
| dictionaries (json objects) | **reference** | shared (alias) |

Guarantee: `ys is xs` where xs is a list/thing NEVER means a copy — changes through
`ys` are visible through `xs`. If you want independence, ask for it explicitly.

### 0.2 `a copy of X`

```text
ks is a copy of xs          # deep copy — the whole tree (nested lists/things)
t2 is a copy of t           # new ThingInstance, same field values (deeply copied)
```

Rules:
1. `a copy of X` is one expression (primary phrase); X parses greedily like the rest
   of the arithmetic (same convention as `the contents of ...`).
2. The copy is DEEP: nested lists/things/dictionaries are copied recursively.
3. Combines freely with `?`: `a copy of maybe?` → nothing if maybe is nothing
   (whole-expression poisoning applies).
4. Modules/functions cannot be copied — friendly error ("a module/function is not a value").
5. Both skins, identical AST (`CopyOf` node; golden 20 + cross-skin pair 8).

Not in the bootstrap cut (arrives in the native pipeline): readable refcounts,
`owned`/move/borrow checking, `unsafe`, cycle collector, `deinit`.

## 1. Three tiers

```text
Default:      ARC (automatic reference counting) + escape analysis
Opt-in:       owned / move / borrow annotations (zero-cost control)
Expert:       unsafe + raw pointers
Optional:     GC cycle collector (--runtime full)
```

## 2. Value types vs reference types

| | struct, enum, tuple, [T;N], primitives | class instance, closures, dyn Trait, Box |
|---|---|---|
| Semantics | value (copy if Copy, else move) | reference (ARC-counted) |
| Layout | inline, stack/embedded | heap-allocated, counted pointer |
| Copy | `.clone()` or Copy | sharing (increment refcount) |

Copy types: all primitives, pointers, arrays/tuples/structs of Copy fields. Copy is opt-out (`@no_copy`). Move invalidates the source (compile error on use after move).

## 3. ARC-detaljer

- Every heap object has a refcount (+ weak count).
- Retain/release are inserted as an IR pass; escape analysis removes pairs within one function.
- Release may inline the destructor; deallocation deterministic at the last release.
- Thread safety: refcounts are atomic by default; single-threaded objects (`@local`) use non-atomic counts (the scheduler guarantees isolation).

### Cycles

- `--runtime full`: an incremental cycle collector (Bacon-Rajan style) runs in the background; latency-bound.
- `--runtime minimal/core`: cycles leak deliberately; `weak` breaks them:

```text
class Node {
    parent: Node?              # strong
    children: Array<Node>
}
n.parent = some(n)             # cycle → collector cleans up in full; otherwise leak
weak_parent: weak Node?        # weak reference; upgrade: .upgrade() -> Node?
```

The linter warns about obvious cycles (static field analysis).

## 4. owned / move / borrow (advanced)

```text
owned buf = Buffer(1 << 20)    # explicit ownership, no refcount
send(buf)                      # MOVE: buf is now invalid

fn process(b: Buffer) {...}    # takes ownership (move in)
fn peek(&buf: Buffer) {}       # borrows, read-only, & = shared borrow
fn edit(mut &buf: Buffer) {}   # mutable borrow
```

The ownership rules (Rust-inspired but **opt-in and milder**):

- A value has one owner; assignment/call = move (for non-Copy).
- Many shared borrows OR one mutable borrow at a time — enforced only for types marked `owned`/`mut &`.
- Ordinary ARC classes can still be shared freely (no borrow checker on them).

This gives Rust-like control where you want it without making the whole language verbose.

## 5. unsafe

```text
unsafe {
    p = malloc(1024)
    defer free(p)
    q = p.offset(8)
    *q = 42u8
    raw = addr_of(x)           # *T
    arr = slice_from_raw(raw, 16)
}
```

- Raw pointers: `*T`, `*mut T`, `null`.
- Unsafe blocks are the only places with deref/addr-of/malloc/FFI.
- unsafe does not contaminate (no unsafe supertype); functions requiring unsafe are marked `@unsafe fn`.
- Debug builds: pointer-sanitizer-style checks (poisoning) where possible.

## 6. Stack and heap

- `[T; N]`, structs, tuples: stack/embedded when possible (escape analysis promotes to heap on escape).
- Large objects (> threshold) go straight to the heap.
- SOO (small object optimization): Optional<T> without an extra tag for reference/niche types.

## 7. Lifetimes

Lifetimes are inferred (non-lexical). Explicit lifetime syntax does NOT exist in v1 — the borrow checker uses regions inference; where impossible, `.clone()` or ARC sharing is required instead. This is a deliberate simplification vs Rust: the language chooses ARC as the escape hatch instead of lifetime annotations.

## 8. Finalization order

1. `defer` (scope exit, LIFO)
2. `use`-dispose (at block end)
3. ARC release → `deinit`
4. Cycle collector (full runtime only, non-deterministic)

## 9. Embedded/minimal profile

- No cycle collector, no reflection metadata, static allocators allowed (`--alloc static`).
- `Box` replaced by a pool allocator in `--alloc arena`.
- Interrupt handlers: `@interrupt fn` (no-allocation verified by the compiler).
