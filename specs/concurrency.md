# Nova Concurrency

## 1. Model overview

```text
Layer 1: async/await      — I/O concurrency on one (or few) threads, M:N tasks
Layer 2: parallel { }     — compiler-managed data parallelism (work-stealing)
Layer 3: spawn/channels   — explicit threads/tasks, message passing
Layer 4: sync primitives  — Mutex, RwLock, Atomics, Condvar, Once, Barrier
Layer 5: GPU             — @gpu kernels (see ARCHITECTURE §backend)
```

Ground principle: **structured concurrency** — child tasks cannot outlive their scope.

## 2. async / await

```text
async fn fetch_json(url: String) -> dynamic {
    resp = await http.get(url)          # suspends the task, not the thread
    json.parse(resp.body_text())
}

async fn main_async() {
    # Sequential
    a = await fetch_json(u1)

    # Concurrent — both start now
    (b, c) = await join(fetch_json(u2), fetch_json(u3))

    # Fault-tolerant race
    first = await any([fetch_json(m1), fetch_json(m2)])     # first success

    # With timeout and cancellation
    d = await timeout(fetch_json(u4), seconds(5))
}

fn main() {
    async_runtime.run(main_async())     # or: top-level await in scripts
}
```

- `await` may only appear inside `async fn`/`async {}` — the effect system enforces this (compile error "blocking call in async context" for synchronous heavier operations marked `@blocking`).
- Async fns lower to state machines (heap-allocated only on escape); zero-cost at non-suspension points.
- `gather`, `join`, `any`, `race`, `timeout`, `sleep`, `yield_now` in `std.async`.
- Cancellation is cooperative: token-based (`ctx.cancelled()?` raises a `Cancelled` Err at checkpoints).

## 3. parallel { }

```text
parallel {
    result_a = calculate_a()       # runs as a task
    result_b = calculate_b()       # runs as a task
}                                   # scope waits for both; values available here
print(result_a + result_b)
```

Semantics:

1. Independent statements become tasks (dependency analysis via dataflow).
2. The scheduler (work-stealing, N=cores) distributes.
3. Scope-exit joins all; exceptions propagate deterministically.
4. Variables written inside the parallel block read afterwards; shared mutable variables across branches = compile error (no data races by construction).

Data parallelism:

```text
results = parallel_map(items, heavy_fn)
parallel_for 0..pixels.len() |i| { render(i) }
sum = xs.parallel().map(f).reduce(0, +)      # parallel iterator pipelines
```

## 4. Threads, tasks, channels

```text
handle = spawn {
    while msg := rx.recv() {
        process(msg)
    }
}

(ch_tx, ch_rx) = Channel<i32>.bounded(64)

ch_tx.send(42)
val = ch_rx.recv()                  # blocking
val = ch_rx.try_recv()              # Result
val = ch_rx.recv_timeout(ms(100))

select {
    x <- ch_rx        => handle(x)
    y <- ch_rx2       => handle(y)
    timeout(sec(1))   => log("timeout")
}

handle.join()
```

- OS threads via `Thread.spawn` (when you really need one); default `spawn` = task on the scheduler.
- Channels: bounded (backpressure), unbounded, oneshot, broadcast, MPSC/SPSC variants.
- `select!` over any number of channels + timers.

## 5. Shared state

```text
counter = Atomic<i64>
counter.fetch_add(1)
counter.load()  counter.store(5)  counter.compare_exchange(0, 1)

lock = Mutex<HashMap<String,i32>>()
use guard = lock.lock() {
    guard["x"] += 1
}

rw = RwLock<Config>()
cfg = rw.read().clone()
```

Rules:

- Data sent between tasks/threads must be `Send`; shared data `Sync` (auto traits, like Rust). The compiler verifies — no data races in safe code.
- Deadlock detection in debug builds (lock-order graph).
- `@local` types are guaranteed single-threaded (non-atomic refcounts, faster).

## 6. VM/JIT integration

- Tasks are cheap (~200 bytes): millions of them are normal.
- One IO reactor per runtime (IOCP/epoll/kqueue); file/net/dns are async-native in stdlib.
- Blocking FFI calls auto-wrap onto a blocking pool.

## 7. Actors — DECIDED

An actor = task + private state + mailbox. Processes **one message at a time** → no locks, data races against the actor's state are impossible.

### 7.1 Syntax

```text
# Natural
a BankAccount is an actor keeping
    a balance of 0

    on deposit with amount
        add amount to my balance
    on withdraw with amount
        if my balance is at least amount then
            take amount from my balance
            reply with "ok"
        otherwise
            reply with "insufficient funds"
    done
done

account is a new BankAccount
send account "deposit" with 100
answer is ask account to "withdraw" with 150     # request/response
```

```text
# Compact
actor BankAccount {
    balance: f64 = 0

    on deposit(amount: f64) { self.balance += amount }
    on withdraw(amount: f64) -> String {
        if self.balance >= amount { self.balance -= amount; "ok" }
        else { "insufficient funds" }
    }
}

acc = BankAccount()
acc.send(.deposit(100))
answer = acc.request(.withdraw(150))      # awaited under the hood in async context
```

### 7.2 Semantics

- Actor instances run as tasks; each actor has a bounded mailbox (same backpressure semantics as channels).
- `send` = fire-and-forget; `request` = awaited reply via a one-shot channel.
- An actor's fields can **only** be accessed from its own `on` handlers — enforced by the compiler (no external field access). All sharing happens via messages.
- Messages must be `Send`; handlers may be `async`.
- Supervision: `link(a, b)` + an `on crash` handler with restart strategies (Erlang-inspired, simple subset).
- Implementation: pure syntactic sugar over Task + Channel + select — no new runtime component.
