# Tasks: meerkat-control-plane-adoption

## Phase 1: Manifest and evidence model

- [x] [serial] r[molten.consensus.algorithm_profile_manifest] Add consensus algorithm profile fields to group manifests, keeping the admitted Raft profile as the default and denying unknown or omitted experimental profiles.
- [x] [parallel] r[molten.consensus.leaderless_profile_boundary] Define the experimental leaderless quorum profile boundary, required policy/proof/simulation evidence, and non-production caveats.
- [x] [parallel] r[molten.consensus.replica_placement_evidence] Add canonical placement report and receipt records for member selection, fault-domain policy, majority reachability assumptions, membership refs, and diagnostics.
- [x] [parallel] r[molten.consensus.non_claim_boundaries] Add denial/readback diagnostics for unsupported Byzantine tolerance, general-database use, ordinary actor traffic, implicit lease reads, and non-control-plane traffic.

## Phase 2: Read and coordination semantics

- [x] [serial] r[molten.consensus.read_consistency_modes] Add explicit control-plane read consistency modes and ensure linearizable reads require read-index or algorithm-specific quorum evidence.
- [x] [serial] r[molten.coordination.read_consistency_modes] Thread the read consistency mode through coordination service requests, receipts, and status assertions.
- [x] [parallel] r[molten.coordination.local_stale_boundaries] Reject local-stale coordination reads wherever mutation guards, lock ownership, fencing, release gates, admission gates, or release evidence require linearizable state.
- [x] [parallel] r[molten.coordination.batched_control_plane_operations] Add canonical batch/CAS-style coordination operation envelopes while preserving per-operation ids, per-operation receipts, and deterministic apply semantics.

## Phase 3: Deterministic simulation and placement tests

- [x] [serial] r[molten.testing.consensus_fault_matrix] Add deterministic consensus simulation fixtures for failed leader, slow leader, concurrent proposals, majority partition progress, minority partition denial, stale linearizable read denial, and stale local read classification.
- [x] [parallel] r[molten.testing.leaderless_experimental_fixtures] Add experimental leaderless fixtures showing majority-connected non-leader progress, constructive concurrent proposal resolution, and denial when required experimental evidence is absent.
- [x] [parallel] r[molten.testing.consensus_placement_fixtures] Add placement fixtures for admitted fault-domain placement, missing placement evidence, unsafe concentration, and membership-policy drift.

## Phase 4: Operator readback and validation

- [x] [serial] r[molten.consensus.algorithm_profile_manifest] Add CLI/readback summaries that show group algorithm profile, read mode support, placement ref, fault-model caveats, and production-readiness status.
- [x] [serial] r[molten.testing.consensus_fault_matrix] Run focused consensus and coordination tests, deterministic simulation tests, Cairn validation, and pre-commit; record pass/fail evidence in implementation notes.

## Implementation notes

Verification completed with `cargo test --lib`, focused consensus/coordination tests for the new paths, `cargo fmt --check`, `pre-commit run --all-files` via the Nix wrapper, and Cairn gates/validation using the canonical Cairn policy because this repo's generated policy JSON is stale and missing `traceability_policy`.
