# Agentic workflow

The repository uses a two-role loop with one deterministic verification harness.

```text
request -> implement skill -> fast checks -> full checks -> review skill -> handoff
              ^                                      |
              +----------- actionable findings -----+
```

## Implement

Invoke `$rustydb-implement` with the requested outcome and acceptance criteria. The implementer inspects the affected vertical slice, changes code and tests, and runs the harness. It leaves a concise handoff containing behavior changed, files changed, verification evidence, and residual risks.

## Review

Invoke `$rustydb-review` with a scope: uncommitted changes, a commit, or a branch/base pair. The reviewer does not modify files. It reports only actionable correctness, compatibility, safety, or test-coverage findings, ordered by severity. An empty review explicitly states that no findings were found and lists remaining test gaps.

## Harness

Run from the repository root:

```console
python scripts/agent_harness.py fast
python scripts/agent_harness.py full
python scripts/agent_harness.py coverage
```

- `fast`: formatting check, Clippy, and tests.
- `full`: `fast` plus `git diff --check`.
- `coverage`: the CI coverage report and per-module threshold. It requires `cargo-llvm-cov`.

The harness stops at the first failure and returns that command's exit code, making it suitable for agents, humans, and CI logs.

