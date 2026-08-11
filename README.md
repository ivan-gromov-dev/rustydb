# RustyDB

RustyDB is a small in-memory key-value database written in Rust. It provides an
interactive command-line interface and a concurrent TCP server inspired by a
focused subset of Redis string, list, set, and expiration commands.

RustyDB begins as a learning-oriented implementation of database internals and
is intended to evolve toward a production-capable system through explicit,
tested guarantees. Current releases remain experimental: data lives only in
process memory and is lost when the application exits.

## Requirements

- Rust 1.85 or newer with the 2024 edition
- Cargo

## Running the database

Run directly from a source checkout:

```console
cargo run
```

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

The server accepts one line-delimited command per line and shares one database
between connected clients. Press Ctrl+C to stop accepting new connections and
wait for active client sessions to finish cleanly.

Each client receives command results without the interactive banner or prompt.
Commands from different clients operate on shared storage, and each complete
command executes atomically under the database lock. The protocol accepts
UTF-8 text only; RESP and binary-safe values are planned for a later release.

Example session:

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

Command names are case-insensitive. Keys cannot contain whitespace. Commands accepting a value preserve spaces inside that value.

## Commands

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
| `HELP` | Print the command list | Help text |
| `EXIT` / `QUIT` | Close the application | `Bye!` |

For `TTL` and `PTTL`, `-1` means the key exists without expiration and `-2` means it does not exist. Expired values are removed lazily when accessed or when collection-wide operations run.

String offsets and lengths are measured in bytes. Negative `GETRANGE` indexes
count backward from the end. When `SETRANGE` starts beyond the current end, the
gap is padded with null bytes (`\0`).

RustyDB stores string, list, and set values. `LPUSH` and `RPUSH` accept the remainder
of the command line as one list element, so an element may contain spaces.
Pushing to an existing list preserves its expiration. List commands applied to
a string, and string or numeric commands applied to a list, return a wrong-type
error without changing the value or its expiration. Popping from a non-empty
list preserves its expiration while values remain; removing the final value
also removes the key.

`LRANGE` uses inclusive indexes. Negative indexes count backward from the end
of the list, and indexes outside the list are clamped to its bounds. An empty
range, missing key, or expired key produces `(nil)`. Reading a range does not
change the list or its expiration.

Set members are unique strings. `SADD`, `SREM`, and `SISMEMBER` accept the
remainder of the command line as one member, so members may contain spaces.
`SMEMBERS` sorts members for deterministic output. Mutating an existing set
preserves its expiration while members remain; removing the final member also
removes the key. Set commands reject strings and lists without mutation.

## Project structure

```text
src/
├── app.rs                 Interactive application loop
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
├── resp/                  RESP2 frame model, encoder, and incremental decoder
├── server/                Concurrent TCP listener and graceful shutdown
└── storage/
    ├── clock.rs           Injectable monotonic clock abstraction
    ├── in_memory.rs       InMemoryStore and StoreError
    ├── indexing.rs        Range-index normalization
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
7. `resp` owns the RESP2 wire-frame model and encoding.
8. `app` provides the interactive loop, while `server` accepts TCP clients and
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

Coverage is aggregated separately for the logical modules `app`, `command`,
`database`, `executor`, `line_protocol`, `line_session`, `output`, `resp`,
`server`, and `storage`. Every module must have more than 70% line coverage.
Test sources and crate bootstrap files are excluded from the per-module
calculation.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the release plan and learning milestones.

## Continuous integration

The GitHub Actions workflow runs formatting and Clippy, every Cargo test target
(including the CLI and TCP integration tests), a real-process Ctrl+C shutdown
test on Linux, and the per-module coverage gate. The final `CI Success` job
succeeds only when all four jobs succeed.

## Current limitations

- RustyDB is experimental and does not yet provide production durability,
  security, availability, or compatibility guarantees.
- No persistence, transactions, authentication, or transport encryption.
- The TCP line protocol is experimental and is not binary-safe or compatible
  with Redis clients.
- Values and keys are held entirely in memory.
- Expiration uses the process monotonic clock and does not survive restarts.

## License

RustyDB is available under the [MIT License](LICENSE).
