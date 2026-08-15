## Context

The harness layer owns suites, reports, schema checks, gate receipts, replay diagnostics, and release evidence. These surfaces are valuable, but they should stay separate from runtime execution and adapter effects.

## Design

### Harness ownership

- `schema`: typed suite/report/gate inputs and parser validation.
- `decision`: pure pass/deny/diagnostic logic for gate evaluation.
- `fixtures`: positive and negative fixture construction for tests.
- `receipts`: canonical harness receipt values and parsers.
- `shell`: filesystem reads, report export, artifact import, and CLI integration.

### Testkit boundary

Reusable harness helpers may move behind a `molten-testkit` or `harness` API that runtime code does not depend on. Runtime code should accept gate receipts or evidence summaries, not instantiate harness runners.

### Validation

Start with a pure gate or schema decision and prove equivalent behavior with valid report fixtures and malformed/stale report fixtures.

## Non-goals

- Do not remove harness commands.
- Do not weaken release evidence gates.
- Do not make diagnostic logs authoritative over canonical receipts.
