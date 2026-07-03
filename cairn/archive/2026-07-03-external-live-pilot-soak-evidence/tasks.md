# Tasks: external-live-pilot-soak-evidence

## Phase 1: Pilot model and runbook

- [x] [serial] r[molten.external_live_pilot_soak.scope_model] Define the constrained pilot scope model with allowed workloads, denied workloads, host identities, rollback triggers, stop-the-line conditions, and evidence-only caveats.
- [x] [parallel] r[molten.external_live_pilot_soak.operator_runbook] Add operator runbook steps for multi-host setup, state roots, live tickets, authority grants, node-control workflow, artifact collection, and teardown.
- [x] [parallel] r[molten.external_live_pilot_soak.evidence_bundle] Define the pilot evidence bundle member set and required child refs.

## Phase 2: Decision core and receipts

- [x] [serial] r[molten.external_live_pilot_soak.decision_receipt] Implement pure pilot decision validation over child evidence refs, decisions, thresholds, scope, caveats, and freshness.
- [x] [parallel] r[molten.external_live_pilot_soak.network_resource_bounds] Bind network diagnostics, resource envelope, SLO/degradation thresholds, and replayability caveats into the pilot decision.
- [x] [parallel] r[molten.external_live_pilot_soak.retention_readback_boundary] Bind retention readback or clearance review evidence without enabling destructive operations by default.

## Phase 3: Positive and negative evidence

- [x] [serial] r[molten.external_live_pilot_soak.positive_workflow] Produce a complete pilot workflow fixture or operator run with node-control, service exchange, blob-ref job, coordination, retention/readback, replay, diagnostics, resource, and rollback child evidence.
- [x] [serial] r[molten.external_live_pilot_soak.negative_denials] Add denial tests for missing peer admission, missing authority, stale ticket, failed replay, diagnostics outside threshold, resource breach, missing retention review, and over-broad pilot scope.
- [x] [serial] r[molten.external_live_pilot_soak.release_readback] Ensure release/pilot readback renders pilot caveats and cannot claim broad production readiness from constrained evidence.

## Phase 4: Validation

- [x] [serial] r[molten.external_live_pilot_soak.validation] Run focused pilot tests, `cargo test`, clippy, Nix nextest, dogfood-local-node, and NixOS VM multi-node baseline before claiming pilot readiness.
