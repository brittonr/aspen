# Tasks: pluggable-consensus-engines

## Phase 1: Registry and interface shape

- [ ] [serial] r[molten.consensus.engine_registry] Add a consensus engine registry model keyed by algorithm profile id and profile version, including implementation id, capability declarations, production-admission status, evidence refs, conformance refs, and caveats.
- [ ] [serial] r[molten.consensus.engine_interface] Define the `ControlPlaneConsensusEngine` boundary with pure decision inputs/outputs for propose, read, snapshot, recovery, membership/config transition, placement validation, and readback summary.
- [ ] [parallel] r[molten.consensus.engine_admission_policy] Add policy/admission checks that deny unknown, disabled, experimental, evidence-incomplete, or capability-mismatched engine entries before runtime construction.

## Phase 2: Runtime selection and Raft adapter

- [ ] [serial] r[molten.consensus.runtime_engine_selection] Replace Raft-only runtime construction with manifest-driven engine selection through the registry while keeping Raft as the only admitted production engine.
- [ ] [parallel] r[molten.consensus.engine_interface] Adapt the existing Raft control-plane path behind the common engine interface without changing existing Raft production semantics.
- [ ] [parallel] r[molten.consensus.engine_portable_state] Ensure control-plane applications consume canonical command/log envelopes and normalized commit/read receipts rather than engine-specific internals.

## Phase 3: Switchover model

- [ ] [serial] r[molten.consensus.engine_switchover_receipts] Add canonical consensus engine switchover plan and receipt records that bind source/target profiles, source state, target bootstrap state, membership/config refs, placement refs, fencing epoch, replay/conformance evidence, currentness evidence, operator approvals, rollback posture, decision, and diagnostics.
- [ ] [serial] r[molten.consensus.engine_switchover_fencing] Deny stale source-engine writes and target-engine reads before activation by checking active engine epoch on mutation and linearizable-read receipts.
- [ ] [parallel] r[molten.coordination.engine_switchover_gates] Thread active engine profile and engine epoch through coordination status/readback and protected-action admission.

## Phase 4: Coordination integration

- [ ] [serial] r[molten.coordination.engine_agnostic_evidence] Update coordination mutation guards, release paths, elections, barriers, rate limits, registry operations, and membership gates to evaluate normalized currentness/fencing evidence instead of Raft-specific fields.
- [ ] [parallel] r[molten.coordination.engine_switchover_gates] Add negative coordination paths for stale engine epoch receipts and not-yet-activated target engine receipts.

## Phase 5: Deterministic conformance and negative fixtures

- [ ] [serial] r[molten.testing.consensus_engine_conformance] Add deterministic per-engine conformance fixtures for proposal, duplicate operation denial, linearizable reads, local-stale reads, snapshot/recovery, membership/config transition denial, canonical replay, and normalized receipt shape.
- [ ] [parallel] r[molten.testing.consensus_registry_negative_fixtures] Add negative registry/admission fixtures for unknown profile, disabled profile, experimental production request, missing conformance refs, missing proof/model evidence, unsupported read mode, version mismatch, missing placement requirements, and unsupported membership/config capability.
- [ ] [parallel] r[molten.testing.consensus_switchover_fixtures] Add deterministic switchover fixtures for safe activation, stale source-state denial, target admission denial, membership incompatibility, placement drift, failed replay/conformance, stale writer fencing, and target read denial before activation.

## Phase 6: Readback and validation

- [ ] [serial] r[molten.consensus.engine_registry] Add CLI/readback summaries for registered engines, active engine profile/version, production status, capability set, evidence refs, conformance refs, and caveats.
- [ ] [serial] r[molten.testing.consensus_engine_conformance] Run focused consensus/coordination tests, deterministic conformance and switchover fixtures, `cargo test --lib`, `cargo fmt --check`, pre-commit, Cairn gates/validation, sync, and archive validation; record pass/fail evidence in implementation notes.
