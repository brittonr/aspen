# Tasks: hegel-core-proof-properties

## Phase 1: Property-law catalog

- [ ] [serial] r[molten.haskell_patterns.hegel_proof_properties.catalog] Define the initial Hegel RS proof-law catalog.
- [ ] [serial] r[molten.haskell_patterns.hegel_proof_properties.canonical_ref_stability] Add canonical ref stability generated tests.
- [ ] [serial] r[molten.haskell_patterns.hegel_proof_properties.traceability_decision_law] Add traceability decision generated tests.

## Phase 2: Negative generated laws

- [ ] [parallel] r[molten.haskell_patterns.hegel_proof_properties.deny_monotonicity] Add deny-monotonicity tests for stale or malformed evidence.
- [ ] [parallel] r[molten.haskell_patterns.hegel_proof_properties.diagnostic_nonpass_law] Add diagnostic/non-pass generated tests.
- [ ] [parallel] r[molten.haskell_patterns.hegel_proof_properties.replay_ref_law] Add replay ref-comparison generated tests.

## Phase 3: Evidence and docs

- [ ] [parallel] r[molten.haskell_patterns.hegel_proof_properties.shrink_fixture_receipts] Persist proof-bound counterexamples as canonical fixtures.
- [ ] [parallel] r[molten.haskell_patterns.hegel_proof_properties.coverage_manifest] Wire Hegel property suites into traceability coverage.
- [ ] [serial] r[molten.haskell_patterns.hegel_proof_properties.docs] Document generator bounds, fixtures, and proof-law expectations.
