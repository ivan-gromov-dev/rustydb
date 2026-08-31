# RustyDB

RustyDB is a small in-memory key-value database written in Rust. It provides an
interactive command-line interface and a concurrent TCP server inspired by a
focused subset of Redis string, list, set, hash, and expiration operations, with
snapshot and append-only persistence.

RustyDB is a learning-oriented implementation of database internals and a
functional engineering demonstration. It is intended to provide an
application-ready standalone subset of Redis rather than production or complete
Redis compatibility. Current releases remain experimental, with snapshot and
append-only persistence available as separate operating modes.

## Requirements

- Rust 1.85 or newer with the 2024 edition
- Cargo

## Running the database

Run directly from a source checkout:

```console
cargo run
```

RustyDB loads `rustydb.snapshot` from the current directory when that file
exists. A missing snapshot starts an empty database. Use an explicit path when
separate instances should keep separate state:

```console
rustydb --snapshot data/example.snapshot
```

Run `SAVE` to write the current state, or request a save after a clean `EXIT` or
end of input:

```console
rustydb --snapshot data/example.snapshot --save-on-shutdown
```

Corrupt, truncated, and unsupported snapshots stop startup with an error rather
than silently starting with incomplete data.

Alternatively, enable the append-only file with an explicit path:

```console
rustydb --aof data/example.aof
```

Every successfully executed mutating command is appended as a binary-safe,
checksummed record and synchronized before its result is acknowledged. RustyDB
replays those records at startup without appending them again. Failed commands
and read-only commands are not recorded. AOF records retain their execution
time, so replay reduces relative `SET`, `GETEX`, `EXPIRE`, and `PEXPIRE`
lifetimes by the time elapsed while RustyDB was stopped. Snapshot options and
`--aof` are currently mutually
exclusive. If a crash leaves the final AOF record incomplete, startup discards
that tail at the previous valid record boundary and continues. A checksum
mismatch or malformed complete record remains a startup error.

Run `AOFREWRITE` in AOF mode to compact command history into the minimum
canonical sequence needed to reproduce the current strings, lists, sets,
hashes, and expirations. The replacement is written and synchronized as a
temporary file before it atomically replaces the previous AOF. Rewriting is
synchronous and holds the shared database lock, so other clients wait until it
completes.

Use `--max-keys` with a positive count to bound the number of stored keys in
interactive or server mode:

```console
rustydb --max-keys 100000
rustydb server 127.0.0.1:6379 --aof data/server.aof --max-keys 100000
```

Overwriting an existing key does not trigger eviction. When a command creates a
key at the limit, RustyDB first reclaims an expired key when one is present;
otherwise it evicts the existing key whose binary key is smallest in
lexicographic byte order. This deliberately simple policy is deterministic and
is not an LRU or LFU approximation. Snapshot loading and AOF replay apply the
configured limit. In AOF mode, evictions are recorded as `DEL` operations so
evicted keys do not reappear after a later restart without the same limit.

Use `--log-level off|error|info|debug` to enable structured operational logs on
standard error. Logging is off by default. `error` records failed commands,
`info` records every completed command, and `debug` additionally records RESP
client connection lifecycle events:

```console
rustydb server --log-level info
```

Log records use a stable space-separated `key=value` format. Command records
contain only the command name and success/error status; stored keys, values,
list elements, set members, hash fields and values, and persistence paths are
never logged.

Or install the binary from a source checkout:

```console
cargo install --path .
rustydb
```

Start the TCP server on the default `127.0.0.1:6379` address:

```console
rustydb server
```

Or provide an explicit bind address:

```console
rustydb server 127.0.0.1:6380
```

Server mode accepts the same persistence options after the optional bind
address:

```console
rustydb server 127.0.0.1:6380 --snapshot data/server.snapshot --save-on-shutdown
```

Use AOF persistence instead by passing `--aof` (snapshot flags cannot be mixed
with it):

```console
rustydb server 127.0.0.1:6380 --aof data/server.aof
```

The server accepts RESP arrays of bulk strings and shares one database between
connected clients. Connections start in RESP2 mode and can switch between RESP2
and RESP3 with `HELLO 2` or `HELLO 3`. It supports fragmented requests,
pipelining, and binary keys and values. Press Ctrl+C to stop accepting new
connections and wait for active client sessions to finish cleanly.

Each client receives command results without the interactive banner or prompt.
Commands from different clients operate on shared storage, and each complete
command executes atomically under the database lock. A malformed RESP frame
closes only its client connection after a protocol error response.
`SAVE` and `AOFREWRITE` also run under that lock, so other clients wait until
the configured persistence operation completes. With `--save-on-shutdown`, the
server first stops accepting connections, waits for active sessions, and then
saves its snapshot.

### Using `redis-cli`

Connect with RESP2 or negotiate RESP3:

```console
redis-cli -2 -h 127.0.0.1 -p 6379
redis-cli -3 -h 127.0.0.1 -p 6379
```

One-shot invocations work for the command subset in the table below, and normal
commands can also be entered in an interactive session. Arguments are sent as
binary-safe bulk strings, so spaces do not require RustyDB-specific parsing.
`redis-cli -x` can supply a binary final argument from standard input:

```console
printf 'line 1\nline 2\0tail' | redis-cli -2 -x SET binary
redis-cli -2 --raw GET binary
```

Run the external-client smoke test after installing `redis-cli`:

```console
python scripts/redis_cli_smoke.py
```

Set `RUSTYDB_REDIS_CLI` to an explicit executable path if `redis-cli` is not on
`PATH`. The smoke test covers connection checks, binary input and echo, strings,
integers, lists, sets, and expiration output through a real client.

RustyDB implements RESP3 response types needed by its documented command subset,
but does not implement authentication, multiple logical databases, transactions,
Pub/Sub, or configuration metadata commands such as `CONFIG`.
Features of `redis-cli` that probe or depend on those commands are not supported.
Interactive `HELP` and `CLEAR` are client-side `redis-cli` commands; use one-shot
invocations to send RustyDB commands with those names. Command errors use
RustyDB's documented messages rather than full Redis error compatibility. See
the official [`redis-cli` documentation](https://redis.io/docs/latest/develop/tools/cli/)
for client installation and option details.

Example interactive CLI session:

```text
Rusty DB
Type HELP to see available commands.
db> SET greeting Hello world
OK
db> GET greeting
Hello world
db> APPEND greeting !
12
db> GET greeting
Hello world!
db> EXIT
Bye!
```

Command names are case-insensitive. In the interactive CLI, keys cannot contain
whitespace and commands accepting a value preserve spaces inside that value.
RESP clients pass every key and value as an exact binary argument.

## Commands

The result column describes the interactive CLI representation. RESP2 and RESP3
clients receive the corresponding protocol-specific typed value.

| Command | Description | Result |
| --- | --- | --- |
| `SET key value [NX\|XX] [GET] [EX seconds\|PX milliseconds\|EXAT unix-seconds\|PXAT unix-milliseconds\|KEEPTTL]` | Store a value with optional existence conditions, old-value return, and expiration policy | `OK`, previous value, or `(nil)` |
| `MSET key value [key value ...]` | Store one or more key/value pairs | `OK` |
| `MSETNX key value [key value ...]` | Atomically store all pairs only when every key is missing | `1` if all were stored, otherwise `0` |
| `SETNX key value` | Store only when the key does not exist | `1` if stored, otherwise `0` |
| `GET key` | Read a value | Value or `(nil)` |
| `GETEX key [EX seconds\|PX milliseconds\|EXAT unix-seconds\|PXAT unix-milliseconds\|PERSIST]` | Read a value and optionally update or remove its expiration | Value or `(nil)` |
| `MGET key [key ...]` | Read multiple values in request order | One value or `(nil)` per line |
| `GETSET key value` | Replace a value and return the previous value | Previous value or `(nil)` |
| `GETDEL key` | Delete a key and return its value | Previous value or `(nil)` |
| `APPEND key value` | Append to a string, creating the key if necessary | New byte length |
| `INCR key` | Increment an integer by one | Updated integer |
| `INCRBY key amount` | Increment an integer by `amount` | Updated integer |
| `DECR key` | Decrement an integer by one | Updated integer |
| `DECRBY key amount` | Decrement an integer by `amount` | Updated integer |
| `INCRBYFLOAT key amount` | Increment a finite floating-point value | Updated number |
| `EXISTS key [key ...]` | Count existing, non-expired keys; duplicate keys are counted repeatedly | Number of matches |
| `DEL key [key ...]` | Delete one or more keys; duplicate keys are removed once | Number deleted |
| `TYPE key` | Report `string`, `list`, `set`, `hash`, or `none` for an expired or missing key | Type name |
| `TOUCH key [key ...]` | Count existing keys (including duplicate arguments); RustyDB has no LRU/LFU access metadata to update | Number of matches |
| `UNLINK key [key ...]` | Delete keys synchronously; duplicate keys are removed once | Number deleted |
| `RENAME old_key new_key` | Move a value and its expiration to another key | `1` if renamed, otherwise `0` |
| `EXPIRE key seconds [NX\|XX\|GT\|LT]` | Set a relative expiration in seconds, optionally subject to a TTL condition | `1` if set, otherwise `0` |
| `PEXPIRE key milliseconds [NX\|XX\|GT\|LT]` | Set a relative expiration in milliseconds, optionally subject to a TTL condition | `1` if set, otherwise `0` |
| `EXPIREAT key unix-seconds [NX\|XX\|GT\|LT]` | Set an absolute Unix expiration in seconds | `1` if set, otherwise `0` |
| `PEXPIREAT key unix-milliseconds [NX\|XX\|GT\|LT]` | Set an absolute Unix expiration in milliseconds | `1` if set, otherwise `0` |
| `TTL key` | Read remaining lifetime in seconds | Remaining TTL, `-1`, or `-2` |
| `PTTL key` | Read remaining lifetime in milliseconds | Remaining TTL, `-1`, or `-2` |
| `EXPIRETIME key` | Read the absolute Unix expiration in seconds | Unix timestamp, `-1`, or `-2` |
| `PEXPIRETIME key` | Read the absolute Unix expiration in milliseconds | Unix timestamp, `-1`, or `-2` |
| `PERSIST key` | Remove an expiration | `1` if removed, otherwise `0` |
| `STRLEN key` | Count bytes in a string | Byte count |
| `GETRANGE key start end` | Read an inclusive byte range | String, possibly empty |
| `SETRANGE key offset value` | Replace bytes starting at an offset | New byte length |
| `LPUSH key value [value ...]` | Prepend one or more values to a list, creating it if necessary | New list length |
| `LPUSHX key value [value ...]` | Prepend values only when the list already exists | New list length, or `0` for a missing key |
| `RPUSH key value [value ...]` | Append one or more values to a list, creating it if necessary | New list length |
| `RPUSHX key value [value ...]` | Append values only when the list already exists | New list length, or `0` for a missing key |
| `LLEN key` | Read a list's length | List length, or `0` for a missing key |
| `LINDEX key index` | Read a list value by zero-based index; negative indexes count from the end | Value or `(nil)` |
| `LSET key index value` | Replace a list value by zero-based index; negative indexes count from the end | `OK` or an error |
| `LINSERT key BEFORE\|AFTER pivot element` | Insert an element relative to the first matching pivot | New length, `0` for a missing key, or `-1` when the pivot is absent |
| `LTRIM key start stop` | Keep only the inclusive list range | `OK` |
| `LREM key count element` | Remove matching elements from the head, tail, or whole list according to `count` | Number removed |
| `LPOS key element [RANK rank] [COUNT count] [MAXLEN len]` | Find matching element indexes with optional occurrence, result-count, and scan limits | Index, indexes, or `(nil)` |
| `LMOVE source destination LEFT\|RIGHT LEFT\|RIGHT` | Atomically move one list element between selected ends | Moved value or `(nil)` |
| `RPOPLPUSH source destination` | Atomically move the source tail to the destination head | Moved value or `(nil)` |
| `LPOP key [count]` | Remove and return the first list value, or up to `count` values | Value, values, or `(nil)` |
| `RPOP key [count]` | Remove and return the last list value, or up to `count` values | Value, values, or `(nil)` |
| `LRANGE key start end` | Read an inclusive list range | Values in list order, or `(nil)` |
| `SADD key member [member ...]` | Add one or more members to a set, creating it if necessary | Number of members added |
| `SREM key member [member ...]` | Remove one or more members from a set | Number of members removed |
| `SISMEMBER key member` | Test whether a set contains a member | `1` if present, otherwise `0` |
| `SMEMBERS key` | Read all set members in sorted order | Members or `(nil)` |
| `SCARD key` | Read a set's cardinality | Number of members, or `0` |
| `HSET key field value [field value ...]` | Set one or more hash fields | Number of newly added fields |
| `HSETNX key field value` | Set a hash field only when it does not exist | `1` if added, otherwise `0` |
| `HGET key field` | Read a hash field | Value or `(nil)` |
| `HMGET key field [field ...]` | Read hash fields in request order | One value or `(nil)` per field |
| `HGETALL key` | Read all hash fields and values in sorted field order | Alternating fields and values, or `(nil)` |
| `HDEL key field [field ...]` | Delete one or more hash fields | Number of fields deleted |
| `HEXISTS key field` | Test whether a hash field exists | `1` if present, otherwise `0` |
| `HLEN key` | Read a hash's field count | Number of fields, or `0` |
| `HKEYS key` | Read hash fields in sorted order | Fields or `(nil)` |
| `HVALS key` | Read hash values ordered by their sorted fields | Values or `(nil)` |
| `HINCRBY key field increment` | Increment an integer hash field | Updated integer |
| `HINCRBYFLOAT key field increment` | Increment a finite floating-point hash field | Updated number |
| `HSCAN key cursor [MATCH pattern] [COUNT count]` | Deterministically inspect sorted hash-field batches | Next cursor followed by field/value pairs |
| `PING [message]` | Test the connection, optionally echoing a binary message | `PONG` or the message |
| `ECHO message` | Return a binary message unchanged | The message |
| `HELLO [2\|3]` | Report connection metadata and optionally select RESP2 or RESP3 | Server metadata |
| `CLIENT ID` | Read the connection's unique, monotonically increasing identifier | Connection ID |
| `CLIENT SETNAME name` | Set or clear the current connection name | `OK` |
| `CLIENT GETNAME` | Read the current connection name | Name or `(nil)` |
| `CLIENT SETINFO LIB-NAME\|LIB-VER value` | Record client library metadata for the connection | `OK` |
| `COMMAND` | List metadata for every supported command in sorted order | Command metadata |
| `COMMAND INFO [command ...]` | Read selected command metadata, or all metadata when omitted | Metadata or `(nil)` per name |
| `COMMAND COUNT` | Count the commands advertised by RustyDB | Command count |
| `SELECT 0` | Select the only supported logical database | `OK`; other indexes are rejected |
| `DBSIZE` | Count non-expired keys in database zero | Number of keys |
| `FLUSHDB [SYNC\|ASYNC]` | Synchronously remove every key from database zero | `OK` |
| `FLUSHALL [SYNC\|ASYNC]` | Synchronously remove every key from the standalone server | `OK` |
| `KEYS pattern` | List non-expired keys matching a binary-safe Redis glob, in sorted order | One key per line or `(nil)` |
| `SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]` | Deterministically inspect sorted keyspace batches; `COUNT` controls examined keys | Next cursor followed by matching keys |
| `RANDOMKEY` | Return a pseudo-random non-expired key | Key or `(nil)` |
| `COPY source destination [DB 0] [REPLACE]` | Copy a value and its remaining TTL, optionally replacing the destination | `1` if copied, otherwise `0` |
| `LEN` | Count non-expired keys | Number of keys |
| `CLEAR` | Remove every key | `OK` |
| `SAVE` | Atomically write the configured snapshot | `OK` or an error |
| `AOFREWRITE` | Atomically compact the configured AOF | `OK` or an error |
| `INFO` | Read runtime counters | One `name:value` counter per line |
| `HELP` | Print the command list | Help text |
| `EXIT` / `QUIT` | Close the current application or connection | `Bye!` in the CLI; `OK` over RESP |

For `TTL` and `PTTL`, `-1` means the key exists without expiration and `-2`
means it does not exist. Expired values are removed lazily when accessed or
when collection-wide operations run. Server mode also performs bounded active-
expiration work between connection accept attempts, reclaiming expired keys
even when clients never access them.

Expiration conditions are mutually exclusive. `NX` applies only when the key
has no expiration, `XX` only when it already has one, and `GT`/`LT` compare the
new deadline with the current deadline. A persistent key is treated as having
an infinite deadline for those comparisons. `EXPIRETIME` and `PEXPIRETIME` use
the same `-1` and `-2` sentinel values as `TTL` and `PTTL`.

Snapshots store expirations as Unix-time millisecond timestamps. Loading turns
future timestamps back into monotonic runtime deadlines and omits keys that
expired while RustyDB was stopped. Snapshot records are ordered for
deterministic output and include a format version and checksum. `SAVE` writes a
temporary file in the destination directory, flushes it, and atomically
replaces the previous snapshot only after the new file is complete.

String offsets and lengths are measured in bytes. Negative `GETRANGE` indexes
count backward from the end. When `SETRANGE` starts beyond the current end, the
gap is padded with null bytes (`\0`).

RustyDB stores string, list, set, and hash values. In the interactive CLI,
`LPUSH` and `RPUSH` accept the remainder of the command line as one list element,
so an element may contain spaces. RESP clients provide the element as one
bulk-string argument.
Pushing to an existing list preserves its expiration. List commands applied to
a string, and string or numeric commands applied to a list, return a wrong-type
error without changing the value or its expiration. For multi-value pushes,
`LPUSH` processes arguments from left to right, so the last value becomes the
list head; `RPUSH` preserves argument order. Popping from a non-empty
list preserves its expiration while values remain; removing the final value
also removes the key.

`LRANGE` uses inclusive indexes. Negative indexes count backward from the end
of the list, and indexes outside the list are clamped to its bounds. An empty
range, missing key, or expired key produces `(nil)`. Reading a range does not
change the list or its expiration.

Set members are unique byte strings. In the interactive CLI, `SADD`, `SREM`, and
`SISMEMBER` accept the remainder of the command line as one member, so members
may contain spaces. RESP clients provide the member as one bulk-string argument.
`SMEMBERS` sorts members for deterministic output. Mutating an existing set
preserves its expiration while members remain; removing the final member also
removes the key. Set commands reject strings and lists without mutation.

Hash fields and values are binary-safe for RESP clients. `HMGET` preserves
request order and duplicate fields. `HGETALL` sorts fields by their binary
representation for deterministic output; RESP3 clients receive a map and RESP2
clients receive an alternating array. Updating a hash preserves its expiration
while fields remain, and deleting its final field removes the key.
`HKEYS`, `HVALS`, and `HSCAN` use the same sorted-field order. As with `SCAN`,
`HSCAN COUNT` controls how many fields are examined, so a filtered batch may
contain fewer results than the requested count.

`INFO` reports counters accumulated since the process started. `connected_clients`
is the number of currently open RESP connections, while `total_connections` is
the number accepted since startup. `commands_processed` includes every parsed
command, including `INFO` and commands that return errors. `keyspace_hits` and
`keyspace_misses` count individual key lookups by `GET`, `MGET`, and `EXISTS`;
duplicate keys count repeatedly. Wrong-type errors count those attempted lookups
as misses. `expired_keys` counts keys reclaimed by lazy, collection-wide, or
active expiration, and `evicted_keys` counts live keys removed to enforce
`--max-keys`. `persistence_successes` and `persistence_failures` count snapshot
saves, AOF rewrites, and individual AOF records, including eviction `DEL`
records. Interactive mode always reports zero connected and total clients.

## Project structure

```text
src/
├── app.rs                 Interactive application loop
├── aof.rs                 Append-only record codec, writer, and replay
├── app/tests.rs           End-to-end CLI-loop tests
├── command/
│   ├── metadata.rs        Deterministic supported-command metadata
│   ├── parser.rs          Text and argument-vector command parser
│   └── types.rs           Command and CommandError types
├── config.rs              Public memory-limit configuration
├── database/              Reusable stateful database service
├── executor/
│   ├── execute.rs         Command dispatch and result mapping
│   └── tests.rs
├── line_protocol.rs       Text-line parsing without execution
├── line_session/          Reusable line-oriented client session
├── output/
│   ├── command_output.rs  Output model and writer-based rendering
│   └── tests.rs
├── resp/                  RESP request codec and RESP2/RESP3 response adapters
├── resp_session/          Buffered RESP request/response session loop
├── server/                Concurrent TCP listener and graceful shutdown
├── snapshot.rs            Versioned snapshot codec and atomic file replacement
└── storage/
    ├── clock.rs           Injectable monotonic clock abstraction
    ├── glob.rs            Binary-safe Redis-style key-pattern matching
    ├── in_memory.rs       InMemoryStore and StoreError
    ├── indexing.rs        Range-index normalization
    ├── snapshot.rs        Snapshot data conversion and TTL restoration
    ├── stored_value.rs    StoredValue and expiration metadata
    ├── value.rs           Typed value representation
    └── tests/              Tests grouped by keys, numbers, strings, lists, sets, hashes, TTL, and values
```

The layers have deliberately narrow responsibilities:

1. `command` validates and converts user input into typed commands.
2. `executor` applies a command to storage and creates a `CommandOutput`.
3. `storage` owns values, numeric operations, ranges, expiration, key limits,
   eviction, and reclamation accounting.
4. `database` owns reusable state and command execution.
5. `line_protocol` and `line_session` coordinate line-oriented parsing and I/O.
6. `output` renders results to any `Write` implementation.
7. `resp` owns RESP request decoding, response encoding, and protocol adapters.
8. `resp_session` coordinates buffered requests, responses, and per-connection
   protocol negotiation.
9. `snapshot` owns point-in-time persistence, while `aof` owns mutation records
   and replay; `storage` converts runtime values and expirations.
10. `app` provides the interactive loop, while `server` accepts TCP clients and
   shares one database between their sessions.

Storage values use an internal enum so new data structures can be added without
changing expiration metadata. Keys, string values, list elements, set members,
and hash fields and values are stored as bytes. Commands currently create
string, list, set, and hash values; operations reject incompatible value kinds
without changing the value
or its TTL.

## Development

Run the fast local verification suite while iterating:

```console
python scripts/agent_harness.py fast
```

Run the end-to-end RESP workload benchmark and see its methodology in
[BENCHMARKS.md](BENCHMARKS.md):

```console
cargo run --release --bin rustydb-benchmark -- --workload mixed --operations 100000 --value-size 64 --concurrency 4
```

Use the opt-in profiling build and platform CPU sampling recipes in
[PROFILING.md](PROFILING.md) to measure allocations and database-lock waiting
without adding instrumentation overhead to normal builds.

Before submitting a change, run the complete suite:

```console
python scripts/agent_harness.py full
```

The full suite runs the following checks:

```console
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff --check
```

To reproduce the CI coverage gate, install
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and run:

```console
python scripts/agent_harness.py coverage
```

The coverage workflow is equivalent to:

```console
cargo llvm-cov --workspace --all-features --json --output-path coverage.json
python scripts/check_module_coverage.py coverage.json --threshold 70
```

Coverage is aggregated separately for the logical modules `aof`, `app`, `command`,
`database`, `executor`, `line_protocol`, `line_session`, `output`, `resp`,
`resp_session`, `server`, `snapshot`, and `storage`. Every module must have more
than 70% line coverage. Test sources and crate bootstrap files are excluded
from the per-module calculation.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the release plan and learning milestones.

## Continuous integration

The GitHub Actions workflow runs formatting and Clippy, every Cargo test target
(including the CLI and TCP integration tests), a release-mode benchmark smoke
test without a throughput threshold, a real-process Ctrl+C shutdown test on
Linux, the external `redis-cli` RESP2/RESP3 smoke test, and the per-module coverage
gate. The final `CI Success` job succeeds only when all five jobs succeed.

## Current limitations

- RustyDB is experimental and does not yet provide production durability,
  security, availability, or compatibility guarantees.
- Snapshot mode can lose mutations after the latest successful `SAVE` unless
  save-on-shutdown completes. AOF mode instead synchronizes each successful
  mutation before acknowledging it.
- No transactions, authentication, or transport encryption.
- Redis protocol compatibility currently covers only the documented command
  subset and the RESP3 response types it requires.
- Live values and keys are held entirely in memory while the process runs.
- `--max-keys` limits key count rather than byte usage; a small number of large
  keys can still consume substantial memory.
- Snapshot format version 2 limits a snapshot to 1,000,000 keys, each list,
  set, or hash to 1,000,000 elements, and each binary field to 512 MiB. Version
  1 snapshots remain readable.
- AOF format version 1 limits one record to 512 MiB and 2,000,001 arguments.

### Intentional scope

RustyDB is intentionally a standalone server with a focused Redis-compatible
feature set. The project does not plan to implement:

- Lua scripting or Redis Functions;
- streams;
- authentication, ACLs, or TLS;
- Redis RDB or multi-part AOF format compatibility;
- replication, Sentinel, or Cluster;
- HyperLogLog or geospatial commands;
- JSON, Search, time-series, vector, probabilistic, or other extended data
  engines;
- multiple logical databases or complete Redis command, protocol, error, and
  operational compatibility.

These are conscious project boundaries rather than untracked future work. See
[ROADMAP.md](ROADMAP.md) for the complete planned feature set.

## License

RustyDB is available under the [MIT License](LICENSE).
