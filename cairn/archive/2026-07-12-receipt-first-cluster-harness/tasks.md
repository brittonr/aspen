## Phase 1: Receipt-first cluster workflow

- [x] [serial] r[molten.testing.receipt_first_cluster_harness.cli_receipt_surface] Add a cluster harness workflow command or `--receipt-out` path that writes `cluster-lifecycle-run-v1` evidence for `init/start/status/stop` phases.
- [x] [parallel] r[molten.testing.receipt_first_cluster_harness.run_artifact_directory] Define the run artifact directory schema and offline verifier over fixture metadata, lifecycle receipts, child refs, drift summaries, diagnostics, and caveats.

## Phase 2: Fixture-backed executable tiers

- [x] [serial] r[molten.testing.receipt_first_cluster_harness.fixture_executable_runner] Wire checked multinode scenario fixture exports into the local multiprocess executable shell before child processes spawn.
- [x] [parallel] r[molten.testing.receipt_first_cluster_harness.fixture_executable_runner] Add positive and negative fixture coverage for isolated state roots, required receipts, stale tickets, child timeout, orphan cleanup, unsupported pass claims, and artifact-kind drift.

## Phase 3: Failure triage and docs

- [x] [serial] r[molten.testing.receipt_first_cluster_harness.failure_triage] Add first-divergence diagnostics and sealed diagnostic failure-bundle export for denied cluster harness runs.
- [x] [parallel] r[molten.testing.receipt_first_cluster_harness.cli_receipt_surface] r[molten.testing.receipt_first_cluster_harness.run_artifact_directory] Document the receipt-first cluster workflow, artifact directory, offline verification command, evidence scopes, and non-claim boundaries.

## Phase 4: Validation

- [x] [serial] [depends:receipt-first-cluster-harness-implementation] r[molten.testing.receipt_first_cluster_harness.cli_receipt_surface] r[molten.testing.receipt_first_cluster_harness.fixture_executable_runner] r[molten.testing.receipt_first_cluster_harness.failure_triage] Run focused cluster lifecycle, local multiprocess, fixture metadata, failure-bundle, and CLI tests.
- [x] [serial] [depends:receipt-first-cluster-harness-validation] r[molten.testing.receipt_first_cluster_harness.run_artifact_directory] Run `nix run path:../cairn#cairn -- validate --root .` plus proposal, design, and tasks gates for this change.
