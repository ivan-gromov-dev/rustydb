# Changelog

All notable changes to RustyDB are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - Unreleased

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

[Unreleased]: https://github.com/Djunichi/rustydb/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Djunichi/rustydb/releases/tag/v0.1.0
