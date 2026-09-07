# Sorted sets: implementation plan for 0.13

Status: implementation in progress. Stages 1-3 are complete; stage 4 iteration,
examples, and final verification remain. This document defines the delivery
sequence and remaining independently verifiable work.

## Data model and invariants

Add a `Value::SortedSet` owned by storage. Start with a member-to-score map and
sort borrowed entries for ordered reads. This keeps one source of truth and
avoids synchronizing two indexes. Member lookup and updates are expected O(1);
ordered reads are O(n log n) and need O(n) temporary references. An order index
can follow measured need rather than being required for this learning project.

Members are unique arbitrary byte strings. Scores are finite `f64` values:
reject NaN and either infinity before mutation, including arithmetic overflow
in `ZINCRBY`. Normalize negative zero to positive zero. A checked score wrapper
should maintain this invariant through command parsing and snapshot decoding;
do not implement `Eq` directly over unchecked floating-point values.

Ascending order compares score first, then member bytes lexicographically.
Reverse order reverses both comparisons. Equal scores never depend on hash-map
iteration order. Updating a member replaces its score without duplicating it.
For duplicate members in one `ZADD`, the final supplied score wins, and the
added-member count counts each newly created member once. Validate every pair
before applying any of them.

Use the existing key expiration and eviction machinery. Reads treat expired
keys as missing; mutations preserve a live key's TTL. Removing the last member
removes the key. Wrong-type and invalid-number failures preserve data and TTL.
`TYPE` and `SCAN TYPE` recognize `zset`; generic copy, rename, deletion, and
key-limit behavior must include the new value type.

## Delivery sequence

Each stage includes parser, executor, output, command metadata, documentation,
and tests for every command it introduces. Do not expose a mutating command
before its AOF serialization and replay are supported.

### 1. Basic commands and durable values

- Add the checked score and sorted-set value, with `ZADD key score member
  [score member ...]`, `ZREM key member [member ...]`, `ZSCORE key member`,
  and `ZCARD key`.
- Limit initial `ZADD` syntax to score/member pairs. Reject unsupported options;
  `NX`, `XX`, `GT`, `LT`, `CH`, and `INCR` are outside this milestone's initial
  subset. Incrementing is supplied separately by `ZINCRBY` in stage 2.
- Return newly added count for `ZADD`, removed count for `ZREM`, an optional
  score for `ZSCORE`, and cardinality for `ZCARD`. Missing keys produce zero
  counts or a null score.
- Add snapshots, AOF replay and rewrite in the same stage, plus generic-key
  operation coverage. Keep the package release version unchanged until release
  preparation.

### 2. Rankings and score updates

Implemented: all commands below, including binary member tie-breaking, explicit
score and member/score output types, RESP2/RESP3 response shapes, TTL preservation,
and `ZINCRBY` AOF replay and rewrite coverage. Single-member rank reads count
predecessors in O(n) time and O(1) auxiliary space. Rank ranges sort borrowed
entries in O(n log n) time with O(n) temporary references. Score-bearing RESP3
responses now use doubles, including `ZSCORE` from stage 1.

- Add `ZMSCORE key member [member ...]`, preserving requested order and nulls;
  `ZRANK key member` and `ZREVRANK key member`, returning zero-based ranks or
  null; and `ZINCRBY key increment member`, treating a missing member as zero.
- Add `ZCOUNT key min max` with inclusive bounds by default, `(` for exclusive
  finite bounds, and `-inf` / `+inf` as unbounded range endpoints. NaN is always
  invalid. Reversed or empty intervals return zero.
- Add rank-based `ZRANGE key start stop [REV] [WITHSCORES]`, with inclusive
  indexes, negative indexes from the selected order's end, and clamping as for
  existing list ranges. Missing keys and empty ranges return empty results.

### 3. Score ranges and queue consumption

Implemented: all commands below, including validation, binary ordering,
protocol-specific pop shapes with/without count, TTL and final-key deletion,
AOF replay/rewrite, snapshot round trips, and concurrent pop coverage.

- Extend `ZRANGE` with `BYSCORE`, `REV`, `LIMIT offset count`, and `WITHSCORES`.
  Score bounds use the same rules as `ZCOUNT`. Under `REV`, the first bound is
  the upper bound. Apply filtering before offset/count. Require a nonnegative
  offset; a negative count means all remaining matches. Reject `LIMIT` in rank
  mode and reject `BYLEX` explicitly.
- Add `ZPOPMIN key [count]` and `ZPOPMAX key [count]`, defaulting to one pair,
  accepting zero, rejecting negative counts, and clamping to cardinality.
  Results remain ordered from the selected end; missing keys return no pairs.
- Add `ZREMRANGEBYRANK key start stop` and `ZREMRANGEBYSCORE key min max`,
  reusing the corresponding range rules and returning removed counts.
- Keep consumption non-blocking in 0.13. A future transaction can combine a due
  score check and removal; a range read followed by removal is not an atomic
  delayed-queue claim across competing clients.

### 4. Iteration and milestone verification

- Add `ZSCAN key cursor [MATCH pattern] [COUNT count]` in binary member order,
  following existing `HSCAN` cursor and examined-entry count conventions.
  Match members, not scores, using the existing binary glob implementation.
  A cursor is not a stable snapshot across writes.
- Document precise interactive, RESP2, and RESP3 response shapes and test them.
  Score-bearing results need explicit output variants: optional individual
  scores, optional score arrays, and member/score pairs must not be flattened
  inside storage. Verify client behavior before claiming Redis compatibility.
- Add executable leaderboard and priority-queue examples and run all module
  coverage gates before marking 0.13 complete.

## Persistence design

The current snapshot writer emits version 2 and reads versions 1 and 2. Write
version 3 with a new value tag `4` for sorted sets, retaining readers for both
older versions. Reject tag `4` in an older-version file. Encode a member count
followed by member blobs and little-endian IEEE-754 score bits in deterministic
member order, then the existing key expiration metadata. Retain collection,
blob, checksum, and allocation limits.

Decode scores through the checked constructor. Reject non-finite scores,
duplicate members, and empty stored collections before replacing live state.
Test old-format fixtures, truncated scores, invalid tags, and corrupt data in
addition to round trips. Signed zero is canonicalized on load.

AOF framing can remain version 1 because it already stores command arguments.
Serialize every new mutator and add it to replay dispatch. Rewrite sorted sets
as deterministic `ZADD` records followed by the existing expiration record.
Use round-tripping decimal score formatting and split records as needed to
respect existing argument and payload limits. Test identical member scores
after replay and rewrite, including finite extremes and fractional values.

## Acceptance coverage

| Area | Required cases |
| --- | --- |
| Parsing | Missing and extra arguments, incomplete pairs, malformed numbers, unsupported or repeated options, no mutation on failure |
| Membership | Insert, update, duplicate input, missing member/key, binary and empty members, final-member deletion |
| Scores | Negative, fractional, zero and signed zero, finite extremes, NaN/infinity rejection, increment overflow |
| Ordering | Equal scores, binary tie-breaking, forward/reverse rank, negative indexes, empty and out-of-range selections |
| Bounds | Inclusive/exclusive endpoints, infinities, equal and reversed bounds, pagination |
| Keys | Wrong type, TTL preserved on success/failure, expiration, copy/rename, active expiration, eviction |
| Persistence | All mutators replay, snapshot restart, AOF rewrite/restart, expiration across downtime, older snapshots and malformed files |
| Frontends | Interactive CLI, binary RESP2/RESP3, metadata and help, multiple clients competing for a pop |

Run `python scripts/agent_harness.py fast` during implementation and `full`
before each handoff; run `coverage` for changed coverage-sensitive logic when
`cargo-llvm-cov` is installed. Add unit tests beside their owning modules and
process tests at the existing CLI/TCP boundaries. Do not relax coverage gates.
