# RustyDB Roadmap

RustyDB is a small learning project and a functional engineering demonstration
of an in-memory database. RustyDB 1.0 reached the intended application-ready
standalone feature set for backend exercises and small test applications.
Future milestones can extend that learning surface without turning RustyDB into
a complete Redis implementation or a production distributed database.

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

## Pull-request checklist

For each feature:

1. Define observable behavior and edge cases.
2. Add or update domain types and errors.
3. Implement storage behavior with focused tests.
4. Connect parsing, execution, output, and persistence where applicable.
5. Add an integration test at the highest available boundary.
6. Run formatting, Clippy, tests, and coverage checks.
7. Update README and this roadmap when the design changes.
