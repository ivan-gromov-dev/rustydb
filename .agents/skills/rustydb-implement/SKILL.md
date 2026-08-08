---
name: rustydb-implement
description: Implement RustyDB features, bug fixes, refactors, tests, or documentation changes with repository-specific architecture and verification. Use when asked to change this repository and deliver a tested implementation. Do not use for read-only review or explanation-only requests.
---

# Implement RustyDB changes

Deliver the smallest complete, verified change.

## Understand

1. Read `AGENTS.md`, the relevant README/roadmap section, source files, and nearby tests.
2. Translate acceptance criteria into observable behavior.
3. Establish baseline behavior. For a defect, reproduce it or write a failing regression test when practical.
4. Check the working tree and preserve unrelated user changes.

## Change

1. Place each responsibility in its owning layer:
   - parse and validate syntax in `command`;
   - dispatch and map results in `executor`;
   - own values, TTL, indexing, and numeric semantics in `storage`;
   - render only in `output`;
   - coordinate I/O in `app`.
2. Keep the crate dependency-free unless the user explicitly approves a dependency.
3. Validate completely before mutation when an operation can fail.
4. Add focused tests for the happy path, relevant boundary, and failure/regression path.
5. Update user-facing documentation when observable behavior changes.

## Verify

1. Run targeted tests during iteration.
2. Run `cargo fmt` after Rust edits.
3. Run `python scripts/agent_harness.py fast`.
4. Inspect the complete diff and run `python scripts/agent_harness.py full` before handoff.
5. If coverage-sensitive logic changed and `cargo-llvm-cov` is available, run `python scripts/agent_harness.py coverage`.

Never hide a failing check. Fix failures caused by the change and distinguish unrelated baseline failures clearly.

## Handoff

Lead with completed behavior. Summarize changed files, verification commands and results, and any remaining risk or intentionally deferred work. Do not claim checks that were not run.
