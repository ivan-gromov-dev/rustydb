# RustyDB 1.0 Guarantees

This document defines the public behavior of RustyDB 1.0. It is a
contract for the standalone server and command-line application, not a claim of
complete Redis compatibility or production-grade availability.

## Compatibility policy

Patch releases preserve documented command syntax, successful response shapes,
persistence readability, and the meanings of documented configuration flags.
They may tighten rejection of malformed input, correct behavior that conflicts
with this document, and add commands or response metadata. A change that makes a
previously valid documented invocation invalid, changes its successful result,
or makes a supported persistence file unreadable requires a new major version.

Exact error text is stable only where README examples or tests document it.
Applications should otherwise treat errors as failures rather than parse their
text. Resource limits may be tightened in a major release.

## Command and concurrency guarantees

- Each non-blocking command executes atomically while holding the shared
  database lock. Other clients cannot observe a partial mutation.
- `EXEC` executes its queued database commands as one atomic batch. An
  execution-time error occupies its result slot and does not prevent later
  queued commands from running.
- A queue-time syntax error aborts the transaction. `DISCARD` and disconnect
  remove queued commands and watches.
- `WATCH` invalidates `EXEC` after another command writes, deletes, expires, or
  evicts a watched key.
- Blocking list commands release the database lock while waiting. A successful
  wake-up and removal or move is atomic, so competing consumers cannot receive
  the same element.
- Result ordering is deterministic where the README specifies an order. Set,
  hash, key-scan, and sorted-set tie ordering is based on binary byte order.

## Expiration and memory guarantees

- Expired keys are not observable as live values. Server mode combines lazy
  expiration with bounded active expiration; interactive mode reclaims through
  lazy and collection-wide access paths.
- Mutations preserve an existing TTL unless the command documentation specifies
  replacement, removal, or an explicit expiration policy.
- Relative expiration is tracked with monotonic time while the process runs.
  Persistence stores wall-clock deadlines so downtime reduces remaining TTL.
- `--max-keys` bounds live key count, not memory usage. At the limit RustyDB
  first reclaims an expired key and otherwise evicts the live key with the
  smallest binary key.

## Persistence guarantees

Snapshot and AOF modes are mutually exclusive.

Snapshot files are point-in-time state. A successful save writes and
synchronizes a temporary file before atomically replacing the destination.
RustyDB 1.0 writes snapshot format version 3 and reads versions 1, 2, and 3.
Malformed, truncated, checksummed-corrupt, unsupported, or oversized snapshots
fail startup instead of partially loading state.

AOF mode synchronizes each successful mutation before acknowledging it. Failed
and read-only commands are not recorded. RustyDB 1.0 writes and reads AOF format
version 1. An incomplete final record is discarded at the previous valid record
boundary; a checksum mismatch or malformed complete record fails startup.
Successful mutations from one `EXEC` are stored as one record, so a truncated
transaction tail is discarded in full. `AOFREWRITE` creates and synchronizes a
replacement before atomically replacing the old file.

These guarantees do not include survival of hardware failure, filesystem or
operating-system bugs, unavailable directory synchronization semantics, or
simultaneous use of one persistence path by multiple RustyDB processes.

## Protocol and operational boundaries

The server accepts RESP arrays of bulk strings, begins in RESP2 mode, and
switches to RESP3 through `HELLO 3`. Requests may be fragmented or pipelined.
A malformed frame closes only its connection after a protocol error response.

Ctrl+C stops new accepts, waits for active sessions, and then performs a
configured save-on-shutdown. Clients that remain connected or blocked can delay
shutdown. Authentication, TLS, replication, clustering, multiple logical
databases, and online configuration are outside the 1.0 scope.

## Failure isolation

- Malformed CLI commands, RESP requests, snapshots, and AOF records are handled
  as errors and do not intentionally panic the process.
- Production library and CLI targets deny direct `unwrap`, `expect`, `panic!`,
  `todo!`, and `unimplemented!` use during Clippy checks. Tests may use these
  constructs to state test invariants.
- Memory-allocation failure, stack exhaustion, standard-library aborts, and
  defects outside RustyDB's explicit error handling are not recoverability
  guarantees.

See [COMPATIBILITY.md](COMPATIBILITY.md) for the supported Redis subset and
[README.md](README.md) for exact command forms and resource limits.
