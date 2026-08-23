# Nova Concurrency

## 1. Model-oversigt

```text
Lag 1: async/await      — I/O-konkurrense på én tråd (eller få), M:N tasks
Lag 2: parallel { }     — compiler-styret dataparallelisme (work-stealing)
Lag 3: spawn/channels   — eksplicitte tråde/tasks, message passing
Lag 4: sync-primitiver  — Mutex, RwLock, Atomics, Condvar, Once, Barrier
Lag 5: GPU             — @gpu kernels (se ARCHITECTURE §backend)
```

Grundprincip: **structured concurrency** — børne-tasks kan ikke overleve deres scope.

## 2. async / await

```text
async fn fetch_json(url: String) -> dynamic {
    resp = await http.get(url)          # suspenderer task, ikke tråd
    json.parse(resp.body_text())
}

async fn main_async() {
    # Sekventielt
    a = await fetch_json(u1)

    # Konkurrentielt — begge starter nu
    (b, c) = await join(fetch_json(u2), fetch_json(u3))

    # Fejltolerant race
    first = await any([fetch_json(m1), fetch_json(m2)])     # første succes

    # Med timeout og cancellation
    d = await timeout(fetch_json(u4), seconds(5))
}

fn main() {
    async_runtime.run(main_async())     # eller: top-level await i scripts
}
```

- `await` kan kun stå i `async fn`/`async {}` — effect-systemet håndhæver det (compile-fejl: "blocking call in async context" for synkrone tungere operationer markeret `@blocking`).
- Async-fns oversættes til state machines (heap-allokeret kun ved flugt); zero-cost ved ikke-suspenderede punkter.
- `gather`, `join`, `any`, `race`, `timeout`, `sleep`, `yield_now` i `std.async`.
- Cancellation er kooperativ: token-baseret (`ctx.cancelled()?` kastes som `Cancelled`-Err ved checkpoints).

## 3. parallel { }

```text
parallel {
    result_a = calculate_a()       # kører som task
    result_b = calculate_b()       # kører som task
}                                   # scope venter på begge; værdier tilgængelige her
print(result_a + result_b)
```

Semantik:

1. Uafhængige statements deklareres til tasks (afhængighedsanalyse via dataflow).
2. Scheduler (work-stealing, N=kerner) fordeler.
3. Scope-exit join'er alle; exceptions propageres deterministisk.
4. Variabler skrevet i parallellblokken læses bagefter; delte mutable variabler på tværs af branches = compile-fejl (ingen dataracer ved konstruktion).

Dataparallelisme:

```text
results = parallel_map(items, heavy_fn)
parallel_for 0..pixels.len() |i| { render(i) }
sum = xs.parallel().map(f).reduce(0, +)      # paralelle iterator-pipelines
```

## 4. Tråde, tasks, channels

```text
handle = spawn {
    while msg := rx.recv() {
        process(msg)
    }
}

(ch_tx, ch_rx) = Channel<i32>.bounded(64)

ch_tx.send(42)
val = ch_rx.recv()                  # blokerende
val = ch_rx.try_recv()              # Result
val = ch_rx.recv_timeout(ms(100))

select {
    x <- ch_rx        => handle(x)
    y <- ch_rx2       => handle(y)
    timeout(sec(1))   => log("timeout")
}

handle.join()
```

- OS-tråde via `Thread.spawn` (når man virkelig skal); default `spawn` = task på scheduleren.
- Channels: bounded (backpressure), unbounded, oneshot, broadcast, MPSC/SPSC-varianter.
- `select!` på vilkårligt mange kanaler + timers.

## 5. Delt tilstand

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

Regler:

- Data sendt mellem tasks/tråde skal være `Send`; delt data `Sync` (auto-traiets, som Rust). Compileren verificerer — ingen dataracer i safe kode.
- Deadlock-detektion i debug-builds (lock-order-graph).
- `@local`-typer er garanteret single-threaded (non-atomare refcounts, hurtigere).

## 6. VM/JIT-integration

- Tasks er billige (~200 bytes): millioner af dem er normalt.
- IO-reactor pr. runtime (IOCP/epoll/kqueue); fil/net/dns er async-native i stdlib.
- Blocking-FFI kald auto-wrappes til blocking pool.

## 7. Actors — BESLUTTET

En actor = task + privat tilstand + mailbox. Behandler **én besked ad gangen** → ingen locks, dataracer umulige mod actorens tilstand.

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
            reply with "ikke dækning"
    done
done

account is a new BankAccount
send account "deposit" with 100
answer is ask account to "withdraw" with 150     # request/response
```

```text
# Kompakt
actor BankAccount {
    balance: f64 = 0

    on deposit(amount: f64) { self.balance += amount }
    on withdraw(amount: f64) -> String {
        if self.balance >= amount { self.balance -= amount; "ok" }
        else { "ikke dækning" }
    }
}

acc = BankAccount()
acc.send(.deposit(100))
answer = acc.request(.withdraw(150))      # await under motorhjelmen i async-kontekst
```

### 7.2 Semantik

- Actor-instanser kører som tasks; hver actor har en bounded mailbox (samme backpressure-semantik som channels).
- `send` = fire-and-forget; `request` = await'et svar via one-shot channel.
- Actorens felter kan **kun** tilgås fra dens egne `on`-handlere — compileren håndhæver det (ingen ekstern felt-adgang). Al deling sker via beskeder.
- Beskeder skal være `Send`; handlers må være `async`.
- Supervision: `link(a, b)` + `on crash`-handler med genstart-strategier (Erlang-inspireret, simpelt subset).
- Implementering: ren syntaktisk sukker over Task + Channel + select — ingen ny runtime-komponent.
