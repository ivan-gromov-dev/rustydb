# RustyDB Troubleshooting

## The server does not start

- Check that the bind address has the form `host:port` and that no other
  process owns the port. Use another local port, for example
  `rustydb server 127.0.0.1:6380`.
- Snapshot and AOF modes are mutually exclusive. Choose either `--snapshot`
  or `--aof`.
- The parent directory of a persistence file must already exist and be
  writable by RustyDB.
- A checksum, version, truncation, or malformed-record error means RustyDB
  refused to load an unsafe persistence file. Preserve that file before
  attempting manual recovery.

## `redis-cli` cannot connect or a command is rejected

- Start RustyDB in server mode; the default interactive mode does not listen on
  TCP.
- Connect to the exact bind address with `redis-cli -h HOST -p PORT`.
- RustyDB supports the focused command forms in the README table, not the
  complete Redis command set. `AUTH`, `CONFIG`, TLS, and databases other than
  zero are intentionally unsupported.
- Arguments from RESP clients are binary-safe. The interactive RustyDB CLI is
  text-oriented and cannot represent whitespace inside keys.

## Shutdown appears to wait

Ctrl+C stops new connections and waits for active sessions before an optional
snapshot save. Close idle clients and clients blocked in `BLPOP`, `BRPOP`, or
`BLMOVE` if shutdown must finish immediately.

## Persistence commands return an error

`SAVE` requires snapshot mode and `AOFREWRITE` requires AOF mode. Verify that
the destination directory exists, has free space, and remains writable. A
failed persistence operation returns an error and does not discard the current
in-memory state; retry only after fixing the filesystem problem.

## Reproducing a problem

Run `rustydb server --log-level debug` for connection and command status logs.
Logs intentionally omit keys and values. For a report, include the RustyDB
version, operating system, complete invocation, client protocol version, error
text, and the smallest command sequence that reproduces the problem. Do not
attach persistence files containing sensitive data.
