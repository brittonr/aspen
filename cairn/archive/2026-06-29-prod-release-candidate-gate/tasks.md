## Phase 1: Candidate evidence contract

- [x] [serial] r[molten.prod_release_candidate.full_validation_matrix] Define and implement the full production release-candidate validation matrix over Rust, nextest, Nix, Cairn, Octet, dogfood, release bundle verification, promotion summary, and export verification evidence.
- [x] [serial] r[molten.prod_release_candidate.source_gate_current] Require current source-gate artifacts after `octet-source-remediated-zero` is complete or explicitly record any remaining configuration-clean caveat as a promotion limiter.

## Phase 2: Gate receipt

- [x] [serial] r[molten.prod_release_candidate.evidence_bundle_promotion] Emit and verify a canonical production release-candidate receipt that binds dogfood, bundle verification, promotion, export, and source-gate refs for the same candidate.
- [x] [serial] r[molten.prod_release_candidate.pilot_decision] Add an explicit pilot-scope decision with allowed workloads, denied workloads, rollback triggers, and evidence-only caveats.

## Phase 3: Docs and tests

- [x] [parallel] r[molten.prod_release_candidate.full_validation_matrix] Add automated pass and stale/mismatched-evidence denial coverage for the release-candidate gate.
- [x] [parallel] r[molten.prod_release_candidate.pilot_decision] Document the operator workflow for producing and reviewing a constrained production-pilot candidate.
