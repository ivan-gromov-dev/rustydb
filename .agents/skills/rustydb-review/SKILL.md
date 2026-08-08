---
name: rustydb-review
description: Review RustyDB working-tree, commit, or branch changes for correctness regressions, command compatibility, expiration and Unicode bugs, unsafe numeric behavior, and missing tests. Use for code review, diff review, pre-commit review, or PR review in this repository. Do not use to implement fixes or modify files.
---

# Review RustyDB changes

Perform a read-only, evidence-driven review.

## Establish scope

1. Read `AGENTS.md` and the relevant documentation and tests.
2. Resolve the requested diff precisely:
   - Uncommitted: inspect `git status --short`, unstaged changes, and staged changes.
   - Commit: inspect the commit and its parent diff.
   - Branch: find the merge base, then diff it against `HEAD`.
3. Include untracked files by reading them directly; ordinary `git diff` omits them.

## Inspect behavior

Trace each changed behavior through `command -> executor -> storage -> output -> app`. Focus on:

- malformed arity and parsing ambiguity;
- mutation before all validation succeeds;
- expired values becoming observable or TTL being lost incorrectly;
- Unicode scalar-value versus byte indexing;
- overflow and non-finite numeric behavior;
- stable ordering and exact CLI output;
- missing boundary, error, and regression tests;
- public behavior that no longer matches README, roadmap, or changelog.

Run the narrowest useful tests when evidence requires it. Run `python scripts/agent_harness.py fast` when the scope is broad enough to justify the full suite. Do not edit files.

## Report

List findings first, ordered by severity. For each finding include severity (`P0` through `P3`), a concise title, a tight file and line range, the concrete trigger and observable impact, and why tests do not prevent it when relevant.

Do not report speculative concerns or formatting enforced by rustfmt/Clippy. If there are no findings, say so explicitly and note residual risks or checks not run.

