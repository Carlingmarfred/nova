# Nova Flow<T> — Design Freeze (v0.20, item N08b)

Status: **FROZEN for v0.20** (lists first). Streams/channels/signals adopt the same
operation vocabulary in later releases; their adapters are stubbed here.

## 1. Vision (unchanged from unique_features U1)

One lazy-sequence concept with one vocabulary across arrays, generators, file lines,
network events and channels. In v0.20 the concrete carrier is the ordinary list.

## 2. v0.2 surface (stdlib `flow`, eager over lists)

```text
use the standard flow library

flow.take(n, xs)        # first n items
flow.skip(n, xs)        # everything after the first n
flow.concat(a, b)       # a ++ b (new list)
flow.flatten(xss)       # one level of nesting removed
flow.unique(xs)         # first occurrence wins (semantic equality)
flow.chunk(xs, n)       # slices of exactly n except possibly the last
```

Rules:
1. Every function returns a NEW list; inputs are never mutated.
2. `take/skip` clamp out-of-range counts instead of raising (Ask-family).
3. `chunk` requires n >= 1; otherwise a friendly sentence error.
4. `unique` preserves first-occurrence order using semantic equality (`nova_eq`).
5. Higher-order operations (`map/filter/fold/reduce`) join when lambdas land (C10);
   they will live in the SAME namespace so call sites never migrate twice.

## 3. Deferred to post-0.2

Lazy/iterating carriers, backpressure for streams/channels, async adapters,
signal history as Flow (needs the temporal engine queries).
