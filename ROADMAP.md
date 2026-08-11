# RustyDB Roadmap

RustyDB begins as a learning-oriented implementation and is intended to evolve
toward a production-capable database. This roadmap first prioritizes
understanding database internals and establishing explicit correctness guarantees
before adding operational maturity and scalability.

Versions describe milestones, not deadlines. Each milestone should be split into
small pull requests and completed with tests and documentation before moving on.
Completing a milestone adds only its documented guarantees; releases remain
experimental until the production-readiness work is complete.

## Guiding principles

- Keep command parsing, execution, storage, and presentation separate.
- Prefer standard-library implementations before introducing frameworks.
- Specify behavior with tests, especially around expiration and failures.
- Measure behavior before optimizing it.
- Document intentional differences from Redis.

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

## Beyond `1.0` — Production readiness

**Goal:** strengthen the stable core with the operational guarantees required
for real deployments.

Expected areas include authentication and transport encryption, resource limits
and backpressure, stable configuration and upgrade paths, backup and restore,
and fault-injection, crash, and stress testing. Replication and high
availability should follow only after the single-node system is dependable and
measurable.

Production readiness is a set of explicit, verified guarantees rather than a
version label. Its detailed milestones will be refined using evidence gathered
while building and operating the releases through `1.0`.

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
