# Nova History Engine — Design Freeze (v0.20, item N08a)

Status: **FROZEN for v0.20** (owner-approved direction; changes require a new
decision-log row). One engine, three consumers.

## 1. Core model

Every `track X` binding owns an append-only snapshot list plus a redo stack:

```text
history[X] : [V0, V1, V2, ...]     # Vn = value BEFORE the n-th mutation? No:
                                    # V0 = initial binding, each Store pushes the NEW value
redo[X]    : [.. popped values ..]
```

Native implementation (vm.rs): `StoreName` pushes the new value when tracked;
`Track` seeds the list with the current value; `Undo` pops history and pushes the
popped value onto redo; `Redo` reverses one undo. Deep copies isolate snapshots.

## 2. Consumers (one engine feeds all)

| Consumer | Surface | Status v0.20 |
|---|---|---|
| Language statements | `track X` / `undo the last change to X` / `redo the last change to X` | ✅ implemented (bootstrap + native) |
| Stdlib queries | `use the standard history library`: `history.snapshots(X)` → list of stored values (oldest first), `history.count(X)` → int | ✅ this freeze |
| Tooling | time-travel debugger (DAP), `nova audit`, `nova why` | future (post-0.2); reads the same lists |

## 3. Rules frozen for v0.20

1. Snapshots are **deep copies**; later mutations never leak into history.
2. Undo/redo never cross function frames (module-local by construction).
3. Untracked names query as empty (`[]` / `0`) — Ask-family semantics.
4. Redo stack clears on any new mutation (classic linear undo).
5. The debugger/audit consumers MUST NOT mutate the lists.

## 4. Deferred (post-0.2)

Temporal query syntax (`ever`, `when did x become Y`, `how many times did ...`),
per-frame isolation switches, persistence.
