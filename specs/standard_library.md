# Nova Standardbibliotek — komplet overflade

Princip: **Python-paritet først** — hver Python-modulekategori har et Nova-modsvar, native implementeret. Derefter Java-agtig enterprise-struktur og C-agtig kontrol.

## 0a. Bootstrap-udsnit (v0.12+, items B03 + C06 + C07 + C08) — B03+C06 IMPLEMENTERET; list + C08-udvidelser følger

`use` binder et ÆGTE navnerums-modul (samme maskineri som C05-moduler):

```text
use the standard json library      # eller: use standard json
say "{json.stringify([1, 2])}"     # navnerums-kald med parenteser
say "{math.PI}"                    # konstanter læses som felt
```

Regler:

1. Formen er `use [the] standard NAVN [library]`. Anden form = venlig fejl der
   viser den rigtige ordlyd. Ukendt NAVN = fejl der lister de tilgængelige.
2. NAVN bliver en almindelig variabel (modul-værdi) — funktioner kaldes
   `NAVN.funktion(arg, ...)`, konstanter læses `NAVN.KONST`. Gen-brug af samme
   bibliotek to gange giver samme instans (ingen dobbelt-init).
3. Fejl i biblioteks-funktioner er almindelige fangbare NovaErrors med linjetal.
4. Bibliotekerne (bootstrap v0):

| Bibliotek | Indhold v0 | Item |
|---|---|---|
| `json` | `parse(text)`, `stringify(v)` | B03 |
| `file` | `read(sti)`, `exists(sti)`, `write(sti, tekst)` | B03 |
| `random` | `between(a, b)`, `pick(liste)`, `shuffle(liste)` (kopi) | B03+C08 |
| `time` | `now()` (sekunder), `sleep(sekunder)` | B03+C08 |
| `math` | `sqrt`, `round`, `abs`, `floor`, `ceil`, `pow(b, e)` + konstant `PI` | B03+C08 |
| `text` | `upper`, `lower`, `trim`, `split(s, sep)`, `join(liste, sep)`, `replace(s, fra, til)`, `length(s)`, `contains(s, sub)`, `at(s, n)` (1-baseret), `slice(s, fra, til)` (1-baseret, inklusiv) | C06 |
| `list` | `sort(liste)`, `reverse(liste)`, `min(liste)`, `max(liste)`, `keys(objekt)`, `values(objekt)` | C07 |

Ikke i bootstrap-udsnittet: `map/filter/fold` (kræver lambdas → C10/T2),
regex/net/http/database og alt andet på M2+ — `use` af ukendt bibliotek fejler
venligt i stedet for at lyve.

## 0. Paritetstabel (Python → Nova)

| Python | Nova | Status |
|---|---|---|
| builtins (len, range, enumerate, zip, ...) | core (indbygget) | M1 |
| str / string | core.String + std.text | M1 |
| list/dict/set/tuple/collections | core.Array/Map/Set/Tuple + std.collections | M1 |
| math / cmath / statistics / random | std.math / std.stats / std.random | M1 |
| os / sys / pathlib / shutil / glob / tempfile | std.fs / std.env / std.process | M1 |
| io / pathlib | std.io | M1 |
| json | std.json | M1 |
| re | std.regex | M2 |
| datetime / zoneinfo / calendar | std.time | M2 |
| itertools / functools | std.iter / std.func | M1/M2 |
| typing / dataclasses / enum | sproget indbygget | M1 |
| logging | std.log | M2 |
| argparse | std.cli | M2 |
| unittest / pytest | indbygget (@test, testkit) | M1 |
| base64 / binascii / hashlib / hmac / secrets / uuid | std.encoding / std.crypto | M2/M4 |
| csv / configparser / tomllib / xml.etree | std.formats.* | M3 |
| sqlite3 (+ DB-API) | std.database (sqlite driver indbygget) | M3 |
| http.client/server / urllib / socket / ssl | std.net.http / std.net.tcp / std.net.tls | M3 |
| socket / select / asyncio | std.async + std.net | M2/M3 |
| threading / multiprocessing / concurrent.futures | std.sync + std.async + parallel | M2 |
| subprocess | std.process | M2 |
| pickle / marshal | std.serialization.binary | M3 |
| array / struct / mmap | std.io.mmap / std.bytes | M3 |
| ctypes / cffi | std.ffi (import c "...") | M2 |
| gc / weakref | std.mem (weak, collect-stats) | M2 |
| decimal / fractions | std.num.Decimal / Rational | M3 |
| gettext / locale | std.i18n | senere |
| tkinter / GUI | nova-gui (officiel pakke) | M5 |
| numpy / pandas | nova-array (officiel pakke, BLAS-backed) | M5 |
| matplotlib | nova-plot (GPU-canvas) | M6 |

## 1. core (indbygget, altid til stede)

```text
print(eprintln debug assert require panic todo unreachable exit)
len first last min max sum sorted reversed abs round clamp
range enumerate zip any all map filter reduce fold
type_of typeof stringify format
Iterable Iterator Optional Result Ordering
Array Map Set Tuple Range Bytes StringBuilder Deque Heap
Comparable Equals Hashable Printable Callable Index Slice Disposable
```

### String (uddrag af fuld API)

```text
len is_empty chars bytes code_points graphemes words lines
lower upper capitalize title swap_case case_fold
trim trim_start trim_end strip(chars)
split rsplit split_once split_lines split_n
join concat repeat pad_left pad_right center truncate
starts_with ends_with contains index_of r_index_of count
replace replace_all remove_prefix remove_suffix
reverse slice substr
to_int to_float to_bool parse::<T>()
encode_utf8 decode(from) escape unescape
compare_options (kultur-følsom kollation via ICU-lite)
format(spec)                     # f"{x:{spec}}"
```

## 2. std.collections

```text
Deque push_front push_back pop_front pop_back rotate extend
Heap push pop peek from_iter heapify
Counter most_common elements total
DefaultMap get_or_insert with_default
FrozenMap FrozenSet (immutable, hashbare)
BitSet rank select count_ones intervals
LRUCache TTLCache (thread-safe varianter: Sync*)
BTreeMap BTreeSet SortedList
RingBuffer CircularBuffer
```

## 3. std.iter

```text
map filter take skip take_while skip_while zip chain interleave
enumerate flat_map flatten scan fuse peekable chunked windowed
step_by rev unique group_by batched pairwise positions
product permutations combinations combinations_with_replacement
cycle repeat repeat_with successions unfold
collect into_array into_map into_set join partition counts
sum_count min_by max_by find_map position all_equal
lazy evaluation: alt er Iterator<T> indtil terminal-operation
```

## 4. std.func

```text
identity compose pipe curry uncurry partial flip memoize(cache_size)
once defer retry(backoff) tap negate constant
```

## 5. std.math

```text
konstanter PI E TAU PHI INF NAN
sqrt cbrt exp exp2 ln log log2 log10 pow hypot
sin cos tan asin acos atan atan2 sinh ...
floor ceil trunc round round_half_even fract
abs sign min max clamp lerp inv_lerp remap smoothstep
factorial gcd lcm isqrt comb perm
fma nextafter epsilon ulp total_cmp
Complex: abs arg conj exp sqrt polar
BigInt: + - * / // % pow gcd modpow to_string parse factorial bit_ops
Rational: exakte brøker, automatisk forkortelse
Decimal: penge-præcis fastpoint, banker-afrunding
checked-aritmetik: add_checked sub_checked ... -> Result
```

## 6. std.stats / std.random

```text
mean median mode variance stdev quantile percentile skewness kurtosis
covariance correlation pearson spearman linregress moving_average z_score
normalisering standardiser histogram bins

Random: default seeded fra OS-entropy; Random(seed) deterministisk (PCG64)
uniform int_range normal log_normal poisson exponential bernoulli binomial
choice choices(weights) shuffle sample(k)
```

## 7. std.fs / std.env / std.process

```text
Path: join parent name ext stem exists is_file is_dir size
     walk(depth-limited) glob("**/*.nova") absolute relative normalize
read_text read_bytes write_text write_bytes append
open(Read/Write/Append, create, truncate, exclusive)
copy move remove mkdir mkdirs rmdir remove_tree
metadata permissions symlink read_link hardlink
tempdir tempfile unique_name watch(events, debounce)

env.get env.set env.delete env.all home_dir cwd exe_path
args (CLI.args) exit_code

Process.run capture shell pipes timeout kill
Pipeline: cmd("git").args(["status"]).pipe(cmd("grep"), "TODO")
```

## 8. std.io

```text
Reader/Writer traits: read fill_buf read_exact read_to_end write flush seek
BufReader BufLineReader BufWriter
stdin stdout stderr
StringWriter BytesWriter
MemoryStream Pipe duplex
mmap(file, mode) — zero-copy file views
compression: gzip zlib deflate brotli zstd (M4)
tar zip archive-API (M4)
```

## 9. std.json

```text
parse(text) -> Result<Json>      Json = dynamic-model
stringify(v, pretty indent sort_keys)
streaming parser/writer (SAX-stil events)
JsonBuilder typed-schema (via reflection eller makro)
JSON-pointer (/a/b/0), JSON Merge Patch, JSON Schema-validering (M4)
```

std.formats: `csv toml yaml ini xml` (M3+), samme builder/streaming-mønster.

## 10. std.regex

RE2-agtig syntaks (ingen backreferences i safe-mode → lineær tid), compile-time-checked literals: `rx"\d{3}-\d{4}"`. Match-groups, named groups, replace, split, scan, global match iterator. Backtracking-motor tilgængelig som opt-in feature.

## 11. std.time

```text
Instant (monotonic) SystemTime Duration TimeSpan
DateTime Date Time TimeZone Calendar
ISO8601/RFC2822/rfc3339 parse/format
aritmetik: dt + days(3); duration.humanize() ("for 3 timer siden")
zoneinfo: IANA-databaser indbygget ("Europe/Copenhagen")
sleep timeout interval stopwatch benchmark-helper
```

## 12. std.log

```text
niveauer trace debug info warn error fatal
log.info("connect til {host}:{port}")
scopes, structured fields (JSON-logning), rotation
handlers: console file syslog net(wire-format)
compile-time niveau-strip i release (--log-level=warn fjerner trace/debug-kald)
```

## 13. std.cli

```text
CLI.app("mit værktøj")
   .arg(required = true, help = "inputfil")
   .option("-o --output", default = "out.txt")
   .flag("-v --verbose", multiple = true)
   .subcommand("build")...
   .parse()

genererer automatisk --help, completion (bash/zsh/fish/ps), man-side
```

## 14. std.net (M3)

```text
tcp: connect listen accept streams (async-native)
udp: sockets multicast
tls: rustls-agtig pure-Nova implementation (M4) + SChannel/OpenSSL-bindinger
dns: resolve async, SRV/TXT
url: parse encode punycode query-params
http: client (HTTP/1.1, HTTP/2, pooling, cookies, redirects, proxies)
      server (routing middleware websocket SSE graceful shutdown)
websocket: client + server
email/smtp ftp ssh(sftp) (senere)
ip: IPv4/IPv6 CIDR
```

Eksempel:

```text
import std.net.http

server = http.Server("0.0.0.0:8080")
server.get("/", ctx => ctx.html("<h1>Nova</h1>"))
server.get("/api/users/{id}", async ctx => {
    user = await db.query("...", [ctx.param("id")])
    ctx.json(user)
})
server.run()
```

## 15. std.database (M3)

```text
DB-API-agtig: connect execute query transaction prepared statements
sqlite3 indbygget (pure binding)
postgres mysql mssql mongodb redis via officielle pakker
ORM-agtig lag via reflection/makroer:

@Table
class User { @id id: i64; name: String }
users = db.select::<User>().where(u => u.age > 18).limit(10).all()
```

## 16. std.crypto (M4)

```text
hash: sha256 sha512 blake3 md5(kun legacy) crc32
mac: hmac poly1305
kdf: pbkdf2 scrypt argon2 hkdf
cipher: aes-gcm chacha20-poly1305 (AEAD-only i safe API)
signaturer: ed25519 ecdsa rsa(pss)
random: CSPRNG (OS-seeded), secrets.token(bytes/url/hex)
constant-time compare overalt
x509/pki hjælpere (M6)
```

## 17. std.serialization (M3)

```text
traits: Serializable Deserializable Schema
formater: json cbor msgpack binary(nova) csv
@derive(Serializable) auto-implementering
versionerede formater (schema-evolution, frem/tilbage-komp.)
```

## 18. std.testing (testkit)

```text
@test fn navn() { expect_eq expect_ne expect_true expect_close
                  expect_throws expect_matches expect_len }
fixtures setup teardown parameterized(@cases)
mocks (makro-baserede), snapshot-testing, property-based (fuzz),
coverage (--coverage), benchmarks (@bench + statistik-rapport)
```

## 19. std.ffi (interop)

```text
import c "sqlite3.h"            # header-import → bindings (libclang-agtig)
@extern("C", lib = "ws2_32")
fn socket(domain: i32, type_: i32, protocol: i32) -> i32

CStrings pointers structs callbacks (unsafe-zone)
Python: import py "numpy" → np; py.eval(...)
Java: import jvm "java.util.ArrayList" (bridge, M6)
WASM: host-functions import/export
COM/WinRT-bindings på Windows (M6)
```

## 20. std.gpu / nova-array / nova-ml / nova-gui (øko-pakker)

```text
@gpu kernels → CUDA/SPIR-V/Metal; device-arrays; map/filter/reduce/matmul
nova-array: ndarray + broadcasting + BLAS/LAPACK + fft (numpy-paritet)
nova-ml: autograd, optimizers, layers, datasets (pytorch-agtig API)
nova-plot: GPU-accelereret plotting (line/scatter/hist/heatmap/3d)
nova-gui: deklarativ widgets, layout, event-loop integreret med async,
          signals-bindinger (label.text binds to ...)
```

## 21. std.units — BESLUTTET

Dimensional analysis (typesystem: type_system.md §12):

```text
basis: m s kg A K mol cd rad sr bit
afledte: N J W Pa Hz C V Ohm T ... (alle kombinationer via Unit-aritmetik)
præfikser: k M G T m µ n p ...
konverteringer: 100.m.in::<km>()  v.in::<km/h>()  2.h.in::<min>()
fysikkonstanter: c g G h e NA R — med korrekt dimension, ikke bare tal
imperiale/enhedssystemer: ft lb mi gal (eksplicit konvertering kun)
valuta: Money<Decimal, "DKK"> — kurser altid eksplicitte .exchange::<USD>(rate)
parse/format: "37.6 km/h".to::<Speed>()
```

## 22. Signals/actors-API-overflade (sprogniveau)

Signals og actors er sprog-features (language_reference §21-22), men eksponerer disse stdlib-navne:

```text
signal(v) computed(fn) effect(fn) unobserve(s) batch { ... }   # atomær multi-opdatering
stream.into_signal() debounce(throttle) sample(period)
actor-supervision: link(a,b) unlink(a) restart_strategy(:one_for_one)
```

## 23. Flow<T> og Table — BESLUTTET (unikke kernetyper)

**Flow<T>** (unique_features.md U1) — én lazy-sekvenstype for alt: Array-view, generatorer, fillinjer, netværks-events, kanaler, signal-historik. Alle iterator-operationer fra §3 virker; async/sync vælges af compileren.

```text
every line of "huge.log" that contains "ERROR"     # Flow<String>, strømmende
repeat for each message in the inbox { ... }        # kanal som Flow
```

**Table** (U2) — kolonne-tabel-primitiv med eget `.ntab`-format:

```text
sales is a table from "sales.csv"
big is the rows of sales where beløb > 1000
per-product is sales grouped by produkt summing beløb
enriched is sales joined-with prices matching product == name
sales.save-as("arkiv.ntab")
```

- Kolonne-layout, SIMD-aggregationer, zero-copy mmap.
- Query-fraserne (§U6) kompilerer til samme plan for Array/Table/Flow/**SQL** (pushdown via std.database).
- `nova why` og time-travel-debuggeren læser samme historik-motor som `track`/undo.
