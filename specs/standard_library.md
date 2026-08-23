# Nova Standard Library — complete surface

Principle: **Python parity first** — every Python module category has a native Nova counterpart. Then Java-style enterprise structure and C-style control.

## 0a. Bootstrap cut (v0.12+, items B03 + C06 + C07 + C08) — FULLY IMPLEMENTED (v0.12)

`use` binds a REAL namespace module (same machinery as C05 modules):

```text
use the standard json library      # or: use standard json
say "{json.stringify([1, 2])}"     # namespace call with parentheses
say "{math.PI}"                    # constants read as a field
```

Rules:

1. The form is `use [the] standard NAME [library]`. Any other form = a friendly error
   that shows the correct wording. Unknown NAME = an error listing the available ones.
2. NAME becomes an ordinary variable (a module value) — functions are called
   `NAME.function(arg, ...)`, constants read as `NAME.CONST`. Re-using the same
   library twice gives the same instance (no double init).
3. Errors in library functions are ordinary catchable NovaErrors with line numbers.
4. The libraries (bootstrap v0):

| Library | Contents v0 | Item |
|---|---|---|
| `json` | `parse(text)`, `stringify(v)` | B03 |
| `file` | `read(path)`, `exists(path)`, `write(path, text)` | B03 |
| `random` | `between(a, b)`, `pick(list)`, `shuffle(list)` (copy) | B03+C08 |
| `time` | `now()` (seconds), `sleep(seconds)` | B03+C08 |
| `math` | `sqrt`, `round`, `abs`, `floor`, `ceil`, `pow(b, e)` + constant `PI` | B03+C08 |
| `text` | `upper`, `lower`, `trim`, `split(s, sep)`, `join(list, sep)`, `replace(s, from, to)`, `length(s)`, `contains(s, sub)`, `at(s, n)` (1-based), `slice(s, from, to)` (1-based, inclusive) | C06 |
| `list` | `sort(list)`, `reverse(list)`, `min(list)`, `max(list)`, `keys(object)`, `values(object)` | C07 |

Not in the bootstrap cut: `map/filter/fold` (requires lambdas → C10/T2),
regex/net/http/database and everything else on M2+ — `use` of an unknown library fails
politely instead of lying.

## 0. Parity table (Python → Nova)

| Python | Nova | Status |
|---|---|---|
| builtins (len, range, enumerate, zip, ...) | core (built-in) | M1 |
| str / string | core.String + std.text | M1 |
| list/dict/set/tuple/collections | core.Array/Map/Set/Tuple + std.collections | M1 |
| math / cmath / statistics / random | std.math / std.stats / std.random | M1 |
| os / sys / pathlib / shutil / glob / tempfile | std.fs / std.env / std.process | M1 |
| io / pathlib | std.io | M1 |
| json | std.json | M1 |
| re | std.regex | M2 |
| datetime / zoneinfo / calendar | std.time | M2 |
| itertools / functools | std.iter / std.func | M1/M2 |
| typing / dataclasses / enum | built into the language | M1 |
| logging | std.log | M2 |
| argparse | std.cli | M2 |
| unittest / pytest | built-in (@test, testkit) | M1 |
| base64 / binascii / hashlib / hmac / secrets / uuid | std.encoding / std.crypto | M2/M4 |
| csv / configparser / tomllib / xml.etree | std.formats.* | M3 |
| sqlite3 (+ DB-API) | std.database (sqlite driver built-in) | M3 |
| http.client/server / urllib / socket / ssl | std.net.http / std.net.tcp / std.net.tls | M3 |
| socket / select / asyncio | std.async + std.net | M2/M3 |
| threading / multiprocessing / concurrent.futures | std.sync + std.async + parallel | M2 |
| subprocess | std.process | M2 |
| pickle / marshal | std.serialization.binary | M3 |
| array / struct / mmap | std.io.mmap / std.bytes | M3 |
| ctypes / cffi | std.ffi (import c "...") | M2 |
| gc / weakref | std.mem (weak, collect-stats) | M2 |
| decimal / fractions | std.num.Decimal / Rational | M3 |
| gettext / locale | std.i18n | later |
| tkinter / GUI | nova-gui (official package) | M5 |
| numpy / pandas | nova-array (official package, BLAS-backed) | M5 |
| matplotlib | nova-plot (GPU-canvas) | M6 |

## 1. core (built-in, always present)

```text
print(eprintln debug assert require panic todo unreachable exit)
len first last min max sum sorted reversed abs round clamp
range enumerate zip any all map filter reduce fold
type_of typeof stringify format
Iterable Iterator Optional Result Ordering
Array Map Set Tuple Range Bytes StringBuilder Deque Heap
Comparable Equals Hashable Printable Callable Index Slice Disposable
```

### String (excerpt of the full API)

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
compare_options (culture-sensitive collation via ICU-lite)
format(spec)                     # f"{x:{spec}}"
```

## 2. std.collections

```text
Deque push_front push_back pop_front pop_back rotate extend
Heap push pop peek from_iter heapify
Counter most_common elements total
DefaultMap get_or_insert with_default
FrozenMap FrozenSet (immutable, hashable)
BitSet rank select count_ones intervals
LRUCache TTLCache (thread-safe variants: Sync*)
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
lazy evaluation: everything is Iterator<T> until a terminal operation
```

## 4. std.func

```text
identity compose pipe curry uncurry partial flip memoize(cache_size)
once defer retry(backoff) tap negate constant
```

## 5. std.math

```text
constants PI E TAU PHI INF NAN
sqrt cbrt exp exp2 ln log log2 log10 pow hypot
sin cos tan asin acos atan atan2 sinh ...
floor ceil trunc round round_half_even fract
abs sign min max clamp lerp inv_lerp remap smoothstep
factorial gcd lcm isqrt comb perm
fma nextafter epsilon ulp total_cmp
Complex: abs arg conj exp sqrt polar
BigInt: + - * / // % pow gcd modpow to_string parse factorial bit_ops
Rational: exact fractions, automatic reduction
Decimal: money-exact fixed point, banker's rounding
checked arithmetic: add_checked sub_checked ... -> Result
```

## 6. std.stats / std.random

```text
mean median mode variance stdev quantile percentile skewness kurtosis
covariance correlation pearson spearman linregress moving_average z_score
normalization standardize histogram bins

Random: default seeded from OS entropy; Random(seed) deterministic (PCG64)
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
tar zip archive API (M4)
```

## 9. std.json

```text
parse(text) -> Result<Json>      Json = dynamic model
stringify(v, pretty indent sort_keys)
streaming parser/writer (SAX-style events)
JsonBuilder typed schema (via reflection or macro)
JSON pointer (/a/b/0), JSON Merge Patch, JSON Schema validation (M4)
```

std.formats: `csv toml yaml ini xml` (M3+), same builder/streaming pattern.

## 10. std.regex

RE2-like syntax (no backreferences in safe mode → linear time), compile-time-checked literals: `rx"\d{3}-\d{4}"`. Match groups, named groups, replace, split, scan, global match iterator. A backtracking engine is available as opt-in.

## 11. std.time

```text
Instant (monotonic) SystemTime Duration TimeSpan
DateTime Date Time TimeZone Calendar
ISO8601/RFC2822/rfc3339 parse/format
arithmetic: dt + days(3); duration.humanize() ("3 hours ago")
zoneinfo: IANA databases built in ("Europe/Copenhagen")
sleep timeout interval stopwatch benchmark helper
```

## 12. std.log

```text
levels trace debug info warn error fatal
log.info("connect to {host}:{port}")
scopes, structured fields (JSON logging), rotation
handlers: console file syslog net(wire-format)
compile-time level stripping in release (--log-level=warn removes trace/debug calls)
```

## 13. std.cli

```text
CLI.app("my tool")
   .arg(required = true, help = "input file")
   .option("-o --output", default = "out.txt")
   .flag("-v --verbose", multiple = true)
   .subcommand("build")...
   .parse()

automatically generates --help, completion (bash/zsh/fish/ps), man page
```

## 14. std.net (M3)

```text
tcp: connect listen accept streams (async-native)
udp: sockets multicast
tls: rustls-like pure-Nova implementation (M4) + SChannel/OpenSSL bindings
dns: resolve async, SRV/TXT
url: parse encode punycode query-params
http: client (HTTP/1.1, HTTP/2, pooling, cookies, redirects, proxies)
      server (routing middleware websocket SSE graceful shutdown)
websocket: client + server
email/smtp ftp ssh(sftp) (later)
ip: IPv4/IPv6 CIDR
```

Example:

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
DB-API-like: connect execute query transaction prepared statements
sqlite3 built-in (pure binding)
postgres mysql mssql mongodb redis via official packages
ORM-like layer via reflection/macros:

@Table
class User { @id id: i64; name: String }
users = db.select::<User>().where(u => u.age > 18).limit(10).all()
```

## 16. std.crypto (M4)

```text
hash: sha256 sha512 blake3 md5(legacy only) crc32
mac: hmac poly1305
kdf: pbkdf2 scrypt argon2 hkdf
cipher: aes-gcm chacha20-poly1305 (AEAD-only in the safe API)
signatures: ed25519 ecdsa rsa(pss)
random: CSPRNG (OS-seeded), secrets.token(bytes/url/hex)
constant-time compare everywhere
x509/pki helpers (M6)
```

## 17. std.serialization (M3)

```text
traits: Serializable Deserializable Schema
formats: json cbor msgpack binary(nova) csv
@derive(Serializable) auto-implementation
versioned formats (schema evolution, forward/backward compat.)
```

## 18. std.testing (testkit)

```text
@test fn name() { expect_eq expect_ne expect_true expect_close
                  expect_throws expect_matches expect_len }
fixtures setup teardown parameterized(@cases)
mocks (macro-based), snapshot testing, property-based (fuzz),
coverage (--coverage), benchmarks (@bench + statistics report)
```

## 19. std.ffi (interop)

```text
import c "sqlite3.h"            # header import → bindings (libclang-like)
@extern("C", lib = "ws2_32")
fn socket(domain: i32, type_: i32, protocol: i32) -> i32

CStrings pointers structs callbacks (unsafe zone)
Python: import py "numpy" → np; py.eval(...)
Java: import jvm "java.util.ArrayList" (bridge, M6)
WASM: host-functions import/export
COM/WinRT bindings on Windows (M6)
```

## 20. std.gpu / nova-array / nova-ml / nova-gui (ecosystem packages)

```text
@gpu kernels → CUDA/SPIR-V/Metal; device-arrays; map/filter/reduce/matmul
nova-array: ndarray + broadcasting + BLAS/LAPACK + fft (numpy parity)
nova-ml: autograd, optimizers, layers, datasets (pytorch-like API)
nova-plot: GPU-accelerated plotting (line/scatter/hist/heatmap/3d)
nova-gui: declarative widgets, layout, event loop integrated with async,
          signals bindings (label.text binds to ...)
```

## 21. std.units — DECIDED

Dimensional analysis (type system: type_system.md §12):

```text
base: m s kg A K mol cd rad sr bit
derived: N J W Pa Hz C V Ohm T ... (all combinations via Unit arithmetic)
prefixes: k M G T m µ n p ...
conversions: 100.m.in::<km>()  v.in::<km/h>()  2.h.in::<min>()
physics constants: c g G h e NA R — with correct dimension, not just numbers
imperial/unit systems: ft lb mi gal (explicit conversion only)
currency: Money<Decimal, "DKK"> — rates always explicit .exchange::<USD>(rate)
parse/format: "37.6 km/h".to::<Speed>()
```

## 22. Signals/actors API surface (language level)

Signals and actors are language features (language_reference §21-22), but they expose these stdlib names:

```text
signal(v) computed(fn) effect(fn) unobserve(s) batch { ... }   # atomic multi-update
stream.into_signal() debounce(throttle) sample(period)
actor supervision: link(a,b) unlink(a) restart_strategy(:one_for_one)
```

## 23. Flow<T> and Table — DECIDED (unique kernel types)

**Flow<T>** (unique_features.md U1) — one lazy sequence type for everything: array views, generators, file lines, network events, channels, signal history. Every iterator operation from §3 works; async/sync is chosen by the compiler.

```text
every line of "huge.log" that contains "ERROR"     # Flow<String>, streaming
repeat for each message in the inbox { ... }        # channel as Flow
```

**Table** (U2) — column-table primitive with its own `.ntab` format:

```text
sales is a table from "sales.csv"
big is the rows of sales where amount > 1000
per-product is sales grouped by product summing amount
enriched is sales joined-with prices matching product == name
sales.save-as("archive.ntab")
```

- Column layout, SIMD aggregations, zero-copy mmap.
- The query phrases (§U6) compile to the same plan for Array/Table/Flow/**SQL** (pushdown via std.database).
- `nova why` and the time-travel debugger read the same history engine as `track`/undo.
