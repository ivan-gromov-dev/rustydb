# RustyDB Roadmap

RustyDB is a small, dependency-free learning project and a functional
engineering demonstration of an in-memory database. The planned destination is
an application-ready standalone server that supports the Redis features commonly
used by backend exercises and small test applications. It is not intended to
become a complete Redis implementation or a production distributed database.

Versions describe milestones, not deadlines. Each milestone should be split into
small pull requests and completed with focused tests and documentation before
moving on.

## Guiding principles

- Keep command parsing, execution, storage, and presentation separate.
- Prefer standard-library implementations before introducing frameworks.
- Preserve binary-safe values, deterministic output, expiration semantics, and
  validation before mutation.
- Implement complete, useful command families instead of accumulating isolated
  commands.
- Document intentional differences from Redis.

## `0.13` — Sorted sets

**Goal:** support leaderboards, rankings, priority queues, and delayed work.

### Work

- Add a sorted-set value with finite floating-point scores and deterministic
  member tie-breaking.
- Add `ZADD`, `ZREM`, `ZSCORE`, `ZMSCORE`, `ZCARD`, `ZRANK`, `ZREVRANK`,
  `ZCOUNT`, and `ZINCRBY`.
- Add `ZRANGE` with rank and score selection needed by the supported workflows.
- Add `ZPOPMIN`, `ZPOPMAX`, `ZREMRANGEBYRANK`, `ZREMRANGEBYSCORE`, and
  `ZSCAN`.
- Extend snapshot and AOF persistence to sorted sets.

### Done when

- Equal scores, inclusive and exclusive score bounds, infinities used as range
  bounds, invalid scores, rank boundaries, TTL, and persistence round trips are
  covered.
- A client can implement a leaderboard and a delayed or priority queue using
  the documented command subset.

## `0.14` — Transactions

**Goal:** provide atomic multi-command workflows and optimistic locking.

### Work

- Add per-client transaction state with `MULTI`, `EXEC`, and `DISCARD`.
- Add `WATCH` and `UNWATCH` using key version tracking.
- Abort watched transactions after writes, deletion, expiration, or eviction of
  a watched key.
- Preserve Redis-style distinctions between queue-time validation errors and
  execution-time command errors.
- Persist an executed transaction as one recoverable logical operation in AOF
  mode.

### Done when

- Commands from another client cannot interleave with an executing transaction.
- Disconnects discard queued commands and watched keys.
- Applications can implement compare-and-set and atomic updates spanning
  multiple keys.

## `0.15` — Publish and subscribe

**Goal:** support local notifications and event-driven test applications.

### Work

- Add `PUBLISH`, `SUBSCRIBE`, `UNSUBSCRIBE`, `PSUBSCRIBE`, and `PUNSUBSCRIBE`.
- Add `PUBSUB CHANNELS` and `PUBSUB NUMSUB`.
- Implement per-client subscription state and RESP2/RESP3 message delivery.
- Define permitted commands while a RESP2 connection is subscribed.
- Remove subscriptions promptly when a client disconnects.

### Done when

- Multiple publishers and subscribers can exchange binary-safe messages without
  blocking unrelated database commands.
- Direct, pattern, unsubscribe, disconnect, and protocol-specific delivery paths
  have integration coverage.

## `1.0` — Verified functional demonstration

**Goal:** finish the planned standalone feature set and make its guarantees,
compatibility, and recovery behavior independently verifiable.

### Work

- Stabilize configuration, supported command behavior, persistence formats, and
  public errors.
- Document architecture, concurrency, durability, expiration, blocking, and
  transaction guarantees.
- Publish a Redis compatibility and intentional-differences matrix.
- Add differential tests that compare the supported command subset with a
  pinned Redis release.
- Add end-to-end crash, recovery, persistence-failure, and multi-client
  scenarios across all supported value types.
- Add parser and persistence fuzzing plus a long randomized workload followed by
  recovery.
- Review public failure paths for unexpected panics.
- Provide complete example applications and troubleshooting guidance.

### Done when

- A new user can build, run, connect with a standard Redis client, exercise each
  documented application pattern, stop, and recover RustyDB using the
  documentation alone.
- The compatibility matrix is backed by automated tests for every claimed
  compatible command.
- CI covers formatting, linting, unit and integration tests, recovery, and the
  supported persistence modes.
- Known limitations and intentional non-goals are explicit.

## Pull-request checklist

For each feature:

1. Define observable behavior and edge cases.
2. Add or update domain types and errors.
3. Implement storage behavior with focused tests.
4. Connect parsing, execution, output, and persistence where applicable.
5. Add an integration test at the highest available boundary.
6. Run formatting, Clippy, tests, and coverage checks.
7. Update README and this roadmap when the design changes.
