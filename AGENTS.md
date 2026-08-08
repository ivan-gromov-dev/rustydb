# RustyDB agent guide

## Mission

- Keep RustyDB a small, dependency-free learning project.
- Preserve the layer boundaries documented in `README.md`: parse in `command`, execute in `executor`, mutate data in `storage`, render in `output`, and wire the loop in `app`.
- Prefer the smallest complete change. Do not add a production dependency without explicit user approval.

## Working loop

1. Read `README.md`, the relevant source and tests, and `ROADMAP.md` when the request changes planned behavior.
2. Establish current behavior before editing. For bugs, add or identify a failing regression test first when practical.
3. Implement through the narrowest appropriate layer. Keep parsing, domain behavior, and presentation separate.
4. Run `cargo fmt` after Rust edits, then `python scripts/agent_harness.py fast`.
5. Before handing off code, run `python scripts/agent_harness.py full`. If a tool is unavailable, report the exact skipped command and why.
6. Review `git diff --check` and the complete diff. Do not overwrite unrelated user changes.

## Rust conventions

- Support stable Rust 1.85+ and edition 2024.
- Keep the crate free of runtime dependencies unless the user explicitly accepts one.
- Avoid `unwrap`, `expect`, and panics in production paths when an error can be represented.
- Preserve Unicode scalar-value semantics for string offsets and lengths.
- Preserve lazy expiration and monotonic-time behavior unless the task explicitly changes it.
- Put unit tests beside their logical module and CLI process tests in `tests/cli.rs`.
- Test success, invalid input, missing keys, boundary values, and overflow/expiration cases relevant to the change.
- Update `README.md`, `ROADMAP.md`, or `CHANGELOG.md` when public behavior, milestones, or release-visible behavior changes.

## Code Review Rules

- Flag parsing that accepts malformed arity or shifts validation into storage/execution.
- Flag changes that make expired keys observable as live values or lose TTL during operations that should preserve it.
- Flag byte-based indexing where RustyDB promises Unicode scalar-value indexing.
- Flag integer/float overflow, non-finite numeric values, and mutation before validation completes.
- Flag nondeterministic output; key listings and multi-value results must retain their documented order.
- Flag production dependencies, public behavior changes without tests/docs, and changes that weaken the per-module coverage gate.
- Findings must identify a concrete failure mode and cite the tightest file/line range. Do not report style-only issues already enforced by rustfmt or Clippy.

## Repo-local skills

- Use `$rustydb-implement` for feature, fix, refactor, or documentation implementation.
- Use `$rustydb-review` for read-only review of a working tree, commit, or branch diff.

