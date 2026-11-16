## Phase 1: Truthful registry receipts

- [x] [serial] Rename the control registry's predicates and checks for what the process computes, require the `in-process-registry` evaluator check, drop quorum claims from the commit predicate and commit receipt, and move predicate, read and commit receipts to v2 with the ledger kind table. r[molten.consensus.trellis_raft_core]
- [x] [serial] Bind registry reads to the `ControlRegistryRuntime`, remove the caller-supplied `read_index`, add `local-committed` with a `registry-committed-read` predicate, keep `local-stale` non-authoritative, and deny `linearizable` with the in-process diagnostic. r[molten.consensus.linearizable_reads] r[molten.consensus.read_consistency_modes]
- [x] [serial] Rename the chain checkpoint control-plane mode to `local-checkpoint` and move checkpoints to v2. r[molten.evidence.chain_hashing.control_plane_checkpoints]

## Phase 2: Coordination and CLI read modes

- [x] [serial] Default coordination, job-schedule and `raft run-fixture` requests to `local-committed`; deny `linearizable` in coordination admission and keep denying `local-stale` for protected operations; replace `read-index-bound` with `committed-state-bound` and move coordination receipts and status assertions to v2. r[molten.coordination.read_consistency_modes] r[molten.coordination.local_stale_boundaries] r[molten.coordination.raft_contract] r[molten.coordination.trellis_predicates] r[molten.coordination.service_registry] r[molten.coordination_services_control_plane.service_registry]
- [x] [serial] Make the in-process descriptor, runtime construction and model switchover advertise only implemented local modes/currentness, preserve the separate algorithm-profile contract, update the v2 artifact fixture, and cover unsupported linearizable engine admission. r[molten.consensus.engine_registry]

## Phase 3: Verification and documentation

- [x] [parallel] Cover linearizable denial, the `local-committed` binding to the committed state, `local-stale` non-authority, protected-operation admission per mode, rejection of v1 receipts and of the `trellis-raft` checkpoint mode. r[molten.consensus.linearizable_reads] r[molten.consensus.read_consistency_modes] r[molten.coordination.read_consistency_modes] r[molten.coordination.local_stale_boundaries] r[molten.evidence.chain_hashing.control_plane_checkpoints]
- [x] [parallel] Correct AGENTS.md, `docs/architecture.md` and `docs/runtime-reference.md` to name the replicated and in-process consensus paths, their callers and read modes, and that neither calls Trellis. r[molten.consensus.trellis_raft_core]
- [x] [serial] Run focused raft-control, coordination, evidence-chain, harness and CLI tests, formatting, Clippy, and strict Cairn validation with the proposal, design and tasks gates before review, sync and archive. r[molten.consensus.trellis_raft_core]

The [earlier complete workspace execution](../../../evidence/integrated-cutover-2026-11-16/earlier-checks.json) passed the library, harness, CLI and doctest selections and retains its later-repaired Clippy failure. The [final ordered source checks](../../../evidence/integrated-cutover-2026-11-16/checks.json) passed the current complete library selection, configured Octet producers/strict admission, Clippy, formatting and strict provider-policy Cairn validation. Local registry receipts do not acquire replicated-Raft or Trellis proof claims.
