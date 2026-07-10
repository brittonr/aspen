# Tasks: cluster-cli-lifecycle-testing

## Phase 1: Lifecycle harness

- [x] [serial] r[molten.testing.cluster_cli.lifecycle_roundtrip] Add a CLI harness test for two-node `cluster init → start → status → stop` using isolated temporary state roots.
- [x] [serial] r[molten.testing.cluster_cli.lifecycle_roundtrip] Assert manifest round-trip, per-node canonical receipt artifacts, already-running start behavior, and reverse stop ordering without relying on stdout as pass evidence.

## Phase 2: Negative fixtures

- [x] [parallel] r[molten.testing.cluster_cli.fail_closed_negatives] Add missing, empty, malformed, unsupported-header, and stale-manifest tests for lifecycle commands.
- [x] [parallel] r[molten.testing.cluster_cli.fail_closed_negatives] Add duplicate/unsafe node name, lifecycle collision, and non-forced reinit denial fixtures.
- [x] [parallel] r[molten.testing.cluster_cli.fail_closed_negatives] Assert `--force` removes only planned node roots and leaves unrelated sibling state untouched.

## Phase 3: Validation

- [x] [serial] r[molten.testing.cluster_cli.lifecycle_roundtrip] Run focused cluster CLI tests and the smallest relevant library tests for cluster planning.
- [x] [serial] r[molten.testing.cluster_cli.fail_closed_negatives] Update traceability coverage with both positive and negative evidence refs.
