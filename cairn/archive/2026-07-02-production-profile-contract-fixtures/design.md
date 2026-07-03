# Design: Production profile contract fixtures

## Context

Nickel contracts should be backed by positive and negative fixture coverage. Positive fixtures guard export compatibility; negative fixtures prove fail-closed behavior for common operator mistakes and adversarial profile edits.

## Fixture layout

Place fixtures near the profile contract or under a test fixture directory with clear names. Keep the checked-in production profile as the canonical positive fixture, and add focused invalid fixtures that change one concern at a time.

Recommended negative fixture classes:

- malformed or unsupported source-gate ref
- empty source-gate input array
- unsafe state root or layout directory
- duplicate or missing required adapter
- unreviewed vocabulary value
- non-positive or fractional resource limit
- contradictory resource relationship
- missing or unsupported schema metadata

## Check runner

Use a deterministic check that exports positive fixtures successfully and expects Nickel export failure for negative fixtures. The runner should record which fixture passed or failed and should not require live network, filesystem state roots, or production credentials.

## Boundaries

Fixtures prove static profile contract behavior. They do not replace runtime startup receipts, source-gate freshness checks, adapter conformance, resource-pressure observations, or operator drill evidence.
