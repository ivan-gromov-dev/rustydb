# Changelog

All notable changes to RustyDB are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/Djunichi/rustydb/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/Djunichi/rustydb/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Djunichi/rustydb/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Djunichi/rustydb/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Djunichi/rustydb/releases/tag/v0.1.0
