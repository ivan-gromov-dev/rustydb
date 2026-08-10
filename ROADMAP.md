# RustyDB Roadmap

RustyDB is a learning project. This roadmap prioritizes understanding database
internals over matching every Redis feature or optimizing prematurely.

Versions describe milestones, not deadlines. Each milestone should be split into
small pull requests and completed with tests and documentation before moving on.

## Guiding principles

- Keep command parsing, execution, storage, and presentation separate.
- Prefer standard-library implementations before introducing frameworks.
- Specify behavior with tests, especially around expiration and failures.
- Measure behavior before optimizing it.
- Document intentional differences from Redis.

## `0.4` — TCP server

**Goal:** make the database accessible to multiple network clients.

### Work

- Extract a reusable database service from the interactive application.
- Add bind-address and port configuration.
- Initially accept commands through a documented line protocol over
  `TcpListener`.
- Handle disconnects and malformed input without terminating the server.
- Start with one client, then support multiple clients with shared storage.
- Define one command under the store lock as the atomicity boundary.
- Add graceful shutdown.

### Done when

- Two clients can safely observe and modify the same database.
- Integration tests start a server on an ephemeral port and use `TcpStream`.
- A bad client cannot crash or poison the server.
- Shutdown stops new connections and lets active work finish cleanly.

**Learning focus:** sockets, threads, synchronization, shared ownership, failure
isolation, and graceful shutdown.

## `0.5` — RESP2 protocol

**Goal:** add a framed, binary-safe protocol compatible with standard Redis
tooling for the supported subset.

### Work

- Model RESP simple strings, errors, integers, bulk strings, arrays, and nulls.
- Build an incremental decoder for partial and multiple frames.
- Encode command output as RESP frames.
- Convert RESP arrays into typed commands.
- Limit frame depth and payload size.
- Document the supported `redis-cli` behavior.

### Done when

- Values can contain whitespace, newlines, and null bytes.
- Fragmented reads and pipelined commands are tested.
- Invalid frames affect only the offending connection.
- Supported commands work through `redis-cli`.

**Learning focus:** wire protocols, byte parsing, buffering, framing, and
defensive input handling.

## `0.6` — Snapshots

**Goal:** preserve state across clean restarts.

### Work

- Define a versioned snapshot format.
- Add `SAVE` and optional save-on-shutdown behavior.
- Save through a temporary file and atomic replacement.
- Load a snapshot at startup.
- Persist expiration as wall-clock timestamps and recreate runtime deadlines.
- Report corrupt, truncated, and unsupported snapshots clearly.

### Done when

- Values, types, and unexpired TTLs survive a restart.
- Keys that expire while stopped are absent after loading.
- A failed save does not destroy the last valid snapshot.
- Round-trip and corruption tests use temporary directories.

**Learning focus:** serialization, filesystem durability, format versioning,
recovery, and monotonic versus wall-clock time.

## `0.7` — Append-only file and recovery

**Goal:** reduce potential data loss and explore write-ahead logging.

### Work

- Append successful mutations to an AOF and replay it at startup.
- Define and document an `fsync` policy.
- Recover safely from a truncated final record.
- Add AOF rewrite/compaction.
- Never record failed commands as successful mutations.

### Done when

- Restart reproduces acknowledged state according to the durability policy.
- Replay does not append replayed commands back into the log.
- Crash and truncation tests cover record boundaries and malformed records.
- Compaction preserves values and TTLs while removing redundant history.

**Learning focus:** write-ahead logs, durability trade-offs, crash recovery, and
log compaction.

## `0.8` — Expiration and memory management

**Goal:** reclaim inaccessible data and control memory growth.

### Work

- Add active expiration alongside lazy expiration.
- Compare sampling with a deadline heap and document the chosen design.
- Handle stale scheduled entries after TTL changes.
- Add configurable key-count or approximate-memory limits.
- Implement one simple eviction policy before considering LRU or LFU.

### Done when

- Unaccessed expired keys are eventually reclaimed.
- Expiration work is bounded and cannot monopolize command execution.
- Limit and eviction behavior is testable.
- Metrics distinguish deletion, expiration, and eviction.

**Learning focus:** scheduling, bounded background work, memory accounting, and
eviction algorithms.

## `0.9` — Observability and performance

**Goal:** measure the system before optimizing it.

### Work

- Add `INFO` counters for clients, commands, hits, misses, expiration, eviction,
  and persistence.
- Add structured logging with configurable verbosity.
- Benchmark common commands and mixed workloads.
- Profile CPU, allocations, and lock contention.
- Optimize only bottlenecks demonstrated by measurements.

### Done when

- Counters have documented meanings and tests.
- Benchmarks record workload, value sizes, concurrency, and environment.
- Performance changes include before-and-after measurements.
- Logs do not expose stored values by default.

**Learning focus:** metrics, benchmarking, profiling, workload design, and
evidence-based optimization.

## `1.0` — Stable educational release

**Goal:** publish a coherent and documented system rather than a collection of
features.

### Work

- Stabilize configuration, command behavior, persistence formats, and errors.
- Document architecture, concurrency, durability, and expiration guarantees.
- Add a Redis compatibility and differences matrix.
- Add end-to-end recovery and multi-client tests.
- Provide example sessions and troubleshooting guidance.
- Review public failure paths for unexpected panics.

### Done when

- A new user can build, run, connect to, stop, and recover RustyDB using the
  documentation alone.
- CI covers formatting, linting, unit and integration tests, and recovery.
- Known limitations and non-goals are explicit.

## Deferred ideas

These topics are intentionally deferred until the milestones above are solid:

- transactions and optimistic locking;
- publish/subscribe;
- authentication and transport encryption;
- asynchronous I/O;
- replication, clustering, and consensus;
- scripting;
- full Redis command and protocol compatibility.

## Pull-request checklist

For each feature:

1. Define observable behavior and edge cases.
2. Add or update domain types and errors.
3. Implement storage behavior with focused tests.
4. Connect parsing, execution, and output.
5. Add an integration test at the highest available boundary.
6. Run formatting, Clippy, tests, and coverage checks.
7. Update README and this roadmap when the design changes.
