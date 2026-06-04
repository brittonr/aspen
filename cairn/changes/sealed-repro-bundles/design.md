# Design: sealed repro bundles

## Context

Reports already contain executable evidence for actor registries, budgets, policy gates, capability gates, admission decisions, effect logs, and replay. Gate checks produce canonical `<gate-receipt-v1 ...>` pass receipts. A repro directory should bind these artifacts into a portable sealed bundle.

## Bundle shape

Report repro bundles keep the existing `<harness-repro-bundle-v1 ...>` envelope and add three fields for sealed bundles:

- `<repro-seal "molten.harness.repro-seal.v1" ...>` with pass decision, report ref, suite ref, profile, replay status, and embedded gate receipt ref.
- Embedded `<gate-receipt-v1 ...>` produced by gating the report artifact.
- `<seal-checks ...>` listing the checked bindings.

The artifact refs list includes the report, suite, initial/final states, actor registry, effect log, policy evidence, capability evidence, budget evidence, UCAN proofset ref, and embedded gate receipt ref.

## Validation

Parsing a sealed bundle checks:

- report ref matches the embedded report hash;
- suite, state, replay/profile, actor-registry, effect-log, and suite values match the embedded report;
- artifact refs contain all report evidence refs;
- seal metadata matches the bundle report metadata;
- embedded gate receipt hashes to the seal receipt ref.

Gate checking a sealed bundle additionally parses the embedded receipt, requires it to be a report receipt bound to the embedded report, recomputes the expected report gate receipt, and compares the exact receipt hash before emitting a new gate receipt for the bundle artifact itself.

## Failure bundles

Failure repro bundles remain diagnostic-only and cannot satisfy pass evidence gates, even if they are exported with command metadata.
