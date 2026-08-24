# RustyDB

RustyDB is a small in-memory key-value database written in Rust. It provides an
interactive command-line interface and a concurrent TCP server inspired by a
focused subset of Redis string, list, set, and expiration operations, with
snapshot and append-only persistence.

RustyDB begins as a learning-oriented implementation of database internals and
is intended to evolve toward a production-capable system through explicit,
tested guarantees. Current releases remain experimental, with snapshot and
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
time, so replay reduces `EXPIRE` and `PEXPIRE` lifetimes by the time elapsed
while RustyDB was stopped. Snapshot options and `--aof` are currently mutually
exclusive. If a crash leaves the final AOF record incomplete, startup discards
that tail at the previous valid record boundary and continues. A checksum
mismatch or malformed complete record remains a startup error.

Run `AOFREWRITE` in AOF mode to compact command history into the minimum
canonical sequence needed to reproduce the current strings, lists, sets, and
expirations. The replacement is written and synchronized as a temporary file
before it atomically replaces the previous AOF. Rewriting is synchronous and
holds the shared database lock, so other clients wait until it completes.

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

The server accepts RESP2 arrays of bulk strings and shares one database between
connected clients. It supports fragmented requests, pipelining, and binary keys
and values. Press Ctrl+C to stop accepting new connections and wait for active
client sessions to finish cleanly.

Each client receives command results without the interactive banner or prompt.
Commands from different clients operate on shared storage, and each complete
command executes atomically under the database lock. A malformed RESP frame
closes only its client connection after a protocol error response.
`SAVE` and `AOFREWRITE` also run under that lock, so other clients wait until
the configured persistence operation completes. With `--save-on-shutdown`, the
server first stops accepting connections, waits for active sessions, and then
saves its snapshot.

### Using `redis-cli`

Force RESP2 when connecting because RustyDB does not implement the RESP3
`HELLO` handshake:

```console
redis-cli -2 -h 127.0.0.1 -p 6379
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
`PATH`. The smoke test covers strings, binary input, integers, lists, sets, and
expiration output through a real client.

RustyDB does not implement RESP3, authentication, database selection,
transactions, Pub/Sub, `SCAN`, or Redis metadata commands such as `COMMAND` and
`INFO`. Features of `redis-cli` that probe or depend on those commands are not
supported. Interactive `HELP` and `CLEAR` are client-side `redis-cli` commands;
use one-shot invocations to send RustyDB commands with those names. Command
errors use RustyDB's documented messages rather than full Redis error
compatibility. See the official [`redis-cli` documentation](https://redis.io/docs/latest/develop/tools/cli/)
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

The result column describes the interactive CLI representation. The RESP2
server returns the corresponding typed RESP value: simple strings, integers,
bulk strings, null bulk strings, or arrays.

| Command | Description | Result |
| --- | --- | --- |
| `SET key value` | Store or overwrite a value and clear its previous expiration | `OK` |
| `MSET key value [key value ...]` | Store one or more key/value pairs | `OK` |
| `SETNX key value` | Store only when the key does not exist | `1` if stored, otherwise `0` |
| `GET key` | Read a value | Value or `(nil)` |
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
| `RENAME old_key new_key` | Move a value and its expiration to another key | `1` if renamed, otherwise `0` |
| `EXPIRE key seconds` | Set expiration in seconds | `1` if set, otherwise `0` |
| `PEXPIRE key milliseconds` | Set expiration in milliseconds | `1` if set, otherwise `0` |
| `TTL key` | Read remaining lifetime in seconds | Remaining TTL, `-1`, or `-2` |
| `PTTL key` | Read remaining lifetime in milliseconds | Remaining TTL, `-1`, or `-2` |
| `PERSIST key` | Remove an expiration | `1` if removed, otherwise `0` |
| `STRLEN key` | Count bytes in a string | Byte count |
| `GETRANGE key start end` | Read an inclusive byte range | String, possibly empty |
| `SETRANGE key offset value` | Replace bytes starting at an offset | New byte length |
| `LPUSH key value` | Prepend a value to a list, creating it if necessary | New list length |
| `RPUSH key value` | Append a value to a list, creating it if necessary | New list length |
| `LLEN key` | Read a list's length | List length, or `0` for a missing key |
| `LPOP key` | Remove and return the first list value | Value or `(nil)` |
| `RPOP key` | Remove and return the last list value | Value or `(nil)` |
| `LRANGE key start end` | Read an inclusive list range | Values in list order, or `(nil)` |
| `SADD key member` | Add a member to a set, creating it if necessary | `1` if added, otherwise `0` |
| `SREM key member` | Remove a member from a set | `1` if removed, otherwise `0` |
| `SISMEMBER key member` | Test whether a set contains a member | `1` if present, otherwise `0` |
| `SMEMBERS key` | Read all set members in sorted order | Members or `(nil)` |
| `SCARD key` | Read a set's cardinality | Number of members, or `0` |
| `KEYS` | List all non-expired keys in sorted order | One key per line or `(nil)` |
| `LEN` | Count non-expired keys | Number of keys |
| `CLEAR` | Remove every key | `OK` |
| `SAVE` | Atomically write the configured snapshot | `OK` or an error |
| `AOFREWRITE` | Atomically compact the configured AOF | `OK` or an error |
| `HELP` | Print the command list | Help text |
| `EXIT` / `QUIT` | Close the current application or connection | `Bye!` in the CLI; `OK` over RESP2 |

For `TTL` and `PTTL`, `-1` means the key exists without expiration and `-2`
means it does not exist. Expired values are removed lazily when accessed or
when collection-wide operations run. Server mode also performs bounded active-
expiration work between connection accept attempts, reclaiming expired keys
even when clients never access them.

Snapshots store expirations as Unix-time millisecond timestamps. Loading turns
future timestamps back into monotonic runtime deadlines and omits keys that
expired while RustyDB was stopped. Snapshot records are ordered for
deterministic output and include a format version and checksum. `SAVE` writes a
temporary file in the destination directory, flushes it, and atomically
replaces the previous snapshot only after the new file is complete.

String offsets and lengths are measured in bytes. Negative `GETRANGE` indexes
count backward from the end. When `SETRANGE` starts beyond the current end, the
gap is padded with null bytes (`\0`).

RustyDB stores string, list, and set values. In the interactive CLI, `LPUSH` and
`RPUSH` accept the remainder of the command line as one list element, so an
element may contain spaces. RESP clients provide the element as one bulk-string
argument.
Pushing to an existing list preserves its expiration. List commands applied to
a string, and string or numeric commands applied to a list, return a wrong-type
error without changing the value or its expiration. Popping from a non-empty
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

## Project structure

```text
src/
├── app.rs                 Interactive application loop
├── aof.rs                 Append-only record codec, writer, and replay
├── app/tests.rs           End-to-end CLI-loop tests
├── command/
│   ├── parser.rs          Text and argument-vector command parser
│   └── types.rs           Command and CommandError types
├── database/              Reusable stateful database service
├── executor/
│   ├── execute.rs         Command dispatch and result mapping
│   └── tests.rs
├── line_protocol.rs       Text-line parsing without execution
├── line_session/          Reusable line-oriented client session
├── output/
│   ├── command_output.rs  Output model and writer-based rendering
│   └── tests.rs
├── resp/                  RESP2 frames, codec, and command/output adapters
├── resp_session/          Buffered RESP2 request/response session loop
├── server/                Concurrent TCP listener and graceful shutdown
├── snapshot.rs            Versioned snapshot codec and atomic file replacement
└── storage/
    ├── clock.rs           Injectable monotonic clock abstraction
    ├── in_memory.rs       InMemoryStore and StoreError
    ├── indexing.rs        Range-index normalization
    ├── snapshot.rs        Snapshot data conversion and TTL restoration
    ├── stored_value.rs    StoredValue and expiration metadata
    ├── value.rs           Typed value representation
    └── tests/              Tests grouped by keys, numbers, strings, lists, sets, TTL, and values
```

The layers have deliberately narrow responsibilities:

1. `command` validates and converts user input into typed commands.
2. `executor` applies a command to storage and creates a `CommandOutput`.
3. `storage` owns values, numeric operations, ranges, and expiration behavior.
4. `database` owns reusable state and command execution.
5. `line_protocol` and `line_session` coordinate line-oriented parsing and I/O.
6. `output` renders results to any `Write` implementation.
7. `resp` owns RESP2 framing, encoding, decoding, and protocol adapters.
8. `resp_session` coordinates buffered RESP2 request and response processing.
9. `snapshot` owns point-in-time persistence, while `aof` owns mutation records
   and replay; `storage` converts runtime values and expirations.
10. `app` provides the interactive loop, while `server` accepts TCP clients and
   shares one database between their sessions.

Storage values use an internal enum so new data structures can be added without
changing expiration metadata. Keys, string values, list elements, and set
members are stored as bytes. Commands currently create string, list, and set
values; operations reject incompatible value kinds without changing the value
or its TTL.

## Development

Run the fast local verification suite while iterating:

```console
python scripts/agent_harness.py fast
```

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
(including the CLI and TCP integration tests), a real-process Ctrl+C shutdown
test on Linux, the external `redis-cli` RESP2 smoke test, and the per-module
coverage gate. The final `CI Success` job succeeds only when all five jobs
succeed.

## Current limitations

- RustyDB is experimental and does not yet provide production durability,
  security, availability, or compatibility guarantees.
- Snapshot mode can lose mutations after the latest successful `SAVE` unless
  save-on-shutdown completes. AOF mode instead synchronizes each successful
  mutation before acknowledging it.
- No transactions, authentication, or transport encryption.
- RESP2 compatibility currently covers only the documented command subset.
- Live values and keys are held entirely in memory while the process runs.
- Snapshot format version 1 limits a snapshot to 1,000,000 keys, each list or
  set to 1,000,000 elements, and each binary field to 512 MiB.
- AOF format version 1 limits one record to 512 MiB and 2,000,001 arguments.

## License

RustyDB is available under the [MIT License](LICENSE).
