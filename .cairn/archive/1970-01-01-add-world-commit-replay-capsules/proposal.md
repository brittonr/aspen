## Why

Molten can compare canonical replay summaries and export bounded simulation bundles. The world-commit roadmap does not yet bind each transition to its expected successor commit or package a complete world closure for independent replay.

A final-state match can hide an earlier divergence. A bundle can also appear portable while omitting one artifact, schema, policy, runtime cohort, snapshot descriptor, or transition input.

## What Changes

- Add canonical world-transition traces that bind an initial commit, ordered transition inputs, and the exact expected commit after every step.
- Add a pure replay planner and verifier that checks complete typed closure before execution.
- Capture the actual successor commit after each replayed step and stop at the earliest identity or typed-root divergence.
- Add canonical world-replay capsule manifests over complete bounded content closure.
- Reuse Molten content manifests, sealed reproduction bundles, and content-exchange adapters instead of creating another transport protocol.
- Validate imported capsules completely before they become available to restore or replay.
- Keep live capabilities, private keys, credentials, mutable heads, and current authority outside capsule content.
- Emit detached replay and import receipts with explicit profile, horizon, and non-claims.

## Dependencies

- `introduce-world-commit-core`.
- `add-world-execution-snapshot-profiles`.
- `add-world-commit-replication-and-retention`.
- Existing Molten replay comparison, sealed reproduction bundles, content-store adapters, and deterministic execution profiles.
- ChaosControl portable snapshot descriptors for opaque replay profiles.

## Non-Goals

- Proof of arbitrary host, kernel, hypervisor, compiler, or runtime determinism.
- Semantic equivalence between logical and opaque snapshot profiles.
- Capability transfer, head movement, effect release, or execution authority from capsule possession.
- A new archive, content transport, replication, or evidence protocol.

## Impact

- **Core**: transition traces, capsule manifests, closure plans, replay comparisons, divergence records, and bounds.
- **Shell**: materialization, profile-specific restore, bounded execution, successor capture, import, and export adapters.
- **Schemas**: canonical Preserves trace, capsule, replay receipt, import receipt, and divergence records.
- **Testing**: complete replay and round-trip cases plus negative missing closure, reordered transition, wrong successor, first-divergence, unsupported profile, tampered object, secret disclosure, and authority-overclaim cases.
