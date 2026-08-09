---
name: rustydb-release-docs
description: Prepare RustyDB documentation immediately before a release by promoting the current changelog work to a dated release, rewriting README content to match the implemented release, and removing completed milestones from ROADMAP. Use for release-documentation preparation, release readiness, or requests to finalize README.md, CHANGELOG.md, and ROADMAP.md for a RustyDB version.
---

# Prepare RustyDB release documentation

Update the three release documents from repository evidence. Preserve changelog
history, describe only verified behavior, and leave the roadmap focused on future
work.

## Establish the release

1. Read `AGENTS.md`, `Cargo.toml`, `README.md`, `CHANGELOG.md`, and `ROADMAP.md`.
2. Inspect `git status`, the changes since the previous release tag, and the
   relevant source and tests. Treat implementation and tests as evidence; do not
   infer features from roadmap plans alone.
3. Determine the target semantic version from the user's request. If omitted,
   infer it only when `Cargo.toml`, changelog content, tags, and repository history
   agree. Stop and ask when they conflict.
4. Use the current local date in `YYYY-MM-DD` form for the release heading unless
   the user supplies another date.

Do not change source code, `Cargo.toml`, create a tag, commit, or publish a release
unless the user separately requests it.

## Finalize CHANGELOG.md

Treat the entries currently collected under `Unreleased` as the target release:

1. Keep an empty `## [Unreleased]` section at the top.
2. Move its release notes under `## [X.Y.Z] - YYYY-MM-DD`.
3. Preserve every older release section.
4. Organize entries using Keep a Changelog categories such as `Added`, `Changed`,
   `Fixed`, `Removed`, and `Known limitations`; include only non-empty categories.
5. Describe observable changes and important architectural preparation concisely.
6. Update comparison links:
   - `[Unreleased]` compares `vX.Y.Z...HEAD`;
   - `[X.Y.Z]` compares the previous release tag with `vX.Y.Z`;
   - preserve older release links.

Never label a release as published merely because a version is planned. In this
workflow, a dated changelog section means the documentation is ready for that
release; tagging and publishing remain separate operations.

## Rewrite README.md for the released state

Review the complete README rather than appending a release summary.

- Make installation, invocation, examples, command tables, edge cases, project
  structure, requirements, verification commands, and limitations agree with the
  implementation at the target release.
- Add material capabilities introduced since the previous release and remove
  obsolete statements.
- Keep internal architecture details only when they help contributors understand
  the current design.
- Do not advertise internal placeholders or commands planned for a later release.
- Keep examples executable and terminology consistent with exact CLI output.
- Prefer a coherent rewrite over scattered sentences that duplicate one another.

README describes the current product, not release history. Put historical changes
in the changelog.

## Prune ROADMAP.md

Remove complete milestone sections through and including the target release. For
example, preparing `0.2.0` removes the `0.1` and `0.2` milestone sections entirely.

- Retain the roadmap introduction, guiding principles, future milestones,
  deferred ideas, and reusable checklist.
- Make the first remaining milestone the next unfinished version.
- Remove references that describe already-completed work as future work.
- Do not copy removed milestone history elsewhere; `CHANGELOG.md` owns it.

## Verify and hand off

1. Review the complete documentation diff for contradictions, stale versions,
   broken headings, and comparison links.
2. Run `git diff --check`.
3. Run `python scripts/agent_harness.py full`; use the configured workspace Python
   when `python` is unavailable on `PATH`.
4. Report the finalized version and date, changed documents, verification results,
   and any release actions intentionally left undone.

Never hide a failing check or claim the release was tagged or published when only
its documentation was prepared.
