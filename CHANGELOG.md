# Changelog

All notable changes to RustyDB are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- An opt-in `--aof path` mode with binary-safe, checksummed mutation records and
  startup replay for interactive and server operation.
- Per-record execution timestamps so replay preserves expiration across
  downtime instead of extending TTLs.
- The `AOFREWRITE` command for atomically compacting history while preserving
  strings, lists, sets, deterministic ordering, and expiration deadlines.

### Changed

- AOF mode synchronizes every successful mutation before acknowledging it and
  never records read-only, failed, or replayed commands.
- Startup truncates an incomplete final AOF record back to the previous valid
  boundary while continuing to reject checksum failures and malformed complete
  records.

## [0.6.0] - 2026-08-17

### Added

- A versioned, checksummed binary snapshot format for string, list, and set
  values, binary keys and data, and absolute expiration timestamps.
- The `SAVE` command for interactive and RESP2 clients.
- Automatic loading from `rustydb.snapshot`, with `--snapshot path` for an
  explicit location and `--save-on-shutdown` for clean CLI and server exits.
- Round-trip, restart, expiration, corruption, failed-replacement, and server
  shutdown coverage using isolated temporary paths.

### Changed

- Snapshot saves now write and synchronize a temporary file in the destination
  directory before atomically replacing the last valid snapshot.
- Snapshot loading recreates monotonic deadlines from wall-clock timestamps and
  omits keys that expired while the process was stopped.
- Corrupt, truncated, unsupported, oversized, and structurally invalid
  snapshots fail startup with explicit errors without partially replacing
  in-memory state.
- Package metadata now reports version 0.6.0.

### Known limitations

- Durability is point-in-time only: acknowledged mutations after the latest
  completed snapshot can be lost after a crash.
- `SAVE` is synchronous and holds the shared database lock while writing, so
  concurrent clients wait for it to finish.

## [0.5.0] - 2026-08-11

### Added

- A dependency-free RESP2 frame model, encoder, and incremental decoder for
  simple strings, errors, integers, bulk strings, arrays, and null values.
- Defensive RESP limits for frame size, array length, and nesting depth.
- Binary-safe RESP request and response adapters for the supported command set.
- A buffered RESP session supporting fragmented reads and pipelined commands in
  request order.
- TCP integration coverage for binary values, pipelining, fragmented frames,
  shared state, graceful shutdown, and malformed-client isolation.
- A real `redis-cli` compatibility smoke test, enforced as a required CI job.

### Changed

- The TCP server now speaks RESP2 arrays of bulk strings instead of the
  line-delimited UTF-8 protocol introduced in 0.4.0.
- Keys, string values, list elements, and set members are stored as arbitrary
  bytes throughout parsing, execution, storage, and output.
- `APPEND`, `STRLEN`, `GETRANGE`, and `SETRANGE` now use Redis-compatible byte
  lengths and offsets.
- Command parsing now shares one validation path between interactive text,
  argument vectors, and RESP requests.
- Protocol errors close only the offending client after an RESP error response;
  valid command errors leave the connection available for later requests.

### Known limitations

- Network compatibility is limited to RESP2 and the documented RustyDB command
  subset; RESP3, inline requests, Redis metadata commands, authentication, and
  database selection are not implemented.

## [0.4.0] - 2026-08-10

### Added

- A reusable `Database` service and line-session layer shared by interactive and
  network frontends.
- A concurrent TCP server with a configurable bind address and documented
  line-delimited command protocol.
- Shared storage across clients with one command as the atomicity boundary.
- Graceful Ctrl+C shutdown that stops new connections and lets active sessions
  finish.
- TCP integration tests, a Linux Ctrl+C process test, and per-module coverage
  enforcement for the new database, protocol, session, and server modules.

### Changed

- Client disconnects, malformed commands, invalid UTF-8, and individual worker
  failures are isolated without terminating or poisoning the server.

## [0.3.0] - 2026-08-10

### Added

- List values with `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LLEN`, and `LRANGE`
  commands.
- Set values with `SADD`, `SREM`, `SISMEMBER`, `SMEMBERS`, and `SCARD`
  commands.

### Changed

- Mutating an existing list preserves its expiration, while list commands
  reject string values without mutation.
- Removing the final value from a list also removes its key.
- Set mutations preserve expiration while members remain, removing the final
  member removes the key, and `SMEMBERS` output is sorted.

## [0.2.0] - 2026-08-09

### Added

- Injectable monotonic clocks for deterministic expiration tests without sleeps
  or wall-clock timing assumptions.
- A typed storage value enum that prepares the data model for additional value
  kinds.
- A `WRONGTYPE`-style storage error for string and numeric operations applied to
  incompatible values.
- A reusable local verification harness for fast, full, and coverage checks.

### Changed

- String commands now access stored values through checked type conversions.
  Failed wrong-type mutations preserve both the existing value and its TTL.

## [0.1.0] - 2026-08-08

### Added

- Interactive, dependency-free command-line database.
- String commands: `SET`, `MSET`, `SETNX`, `GET`, `MGET`, `GETSET`, `GETDEL`,
  `APPEND`, `STRLEN`, `GETRANGE`, and `SETRANGE`.
- Integer and floating-point increment and decrement commands with validation and
  overflow handling.
- Key commands: `EXISTS`, `DEL`, `RENAME`, `KEYS`, `LEN`, and `CLEAR`.
- Lazy expiration through `EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, and `PERSIST`.
- `HELP`, `EXIT`, and `QUIT` application commands.
- Layered command, execution, storage, output, and application design.
- Unit, application, and black-box CLI tests.
- CI checks for formatting, Clippy, tests, and per-module coverage.

### Known limitations

- Data is stored only in process memory and is lost on exit.
- RustyDB has no networking, concurrency, persistence, authentication, or
  transactions.
- Expiration is lazy and uses a process-local monotonic clock.
- The command language is not compatible with the Redis protocol.
- Keys cannot contain whitespace, and values are Unicode strings rather than
  binary-safe byte sequences.

[Unreleased]: https://github.com/Djunichi/rustydb/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/Djunichi/rustydb/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Djunichi/rustydb/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Djunichi/rustydb/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Djunichi/rustydb/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Djunichi/rustydb/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Djunichi/rustydb/releases/tag/v0.1.0
