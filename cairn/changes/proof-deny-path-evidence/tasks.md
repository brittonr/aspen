# Tasks: proof-deny-path-evidence

## Phase 1: Matrix and receipts

- [ ] [serial] r[molten.evidence.proof_deny_matrix.catalog] Define a deny-path matrix model for proof-bearing gates.
- [ ] [serial] r[molten.evidence.proof_deny_matrix.fail_closed_fixtures] Enumerate required negative fixture classes for supported proof gates.
- [ ] [serial] r[molten.evidence.proof_deny_matrix.no_mutation_evidence] Bind no-mutation evidence for denials that happen before side effects.

## Phase 2: Gate integration

- [ ] [parallel] r[molten.evidence.proof_deny_matrix.schema_tamper_cases] Add schema/tamper denial coverage to relevant gates.
- [ ] [parallel] r[molten.evidence.proof_deny_matrix.signature_tamper_cases] Add signer, purpose, key, and duplicate denial coverage where signed evidence is accepted.
- [ ] [parallel] r[molten.evidence.proof_deny_matrix.diagnostic_only] Ensure diagnostic-only evidence cannot satisfy pass gates.

## Phase 3: Hegel RS and docs

- [ ] [parallel] r[molten.evidence.proof_deny_matrix.hegel_properties] Add Hegel RS generated tests for stale refs, malformed schemas, duplicate receipts, and denied mutations.
- [ ] [serial] r[molten.evidence.proof_deny_matrix.docs] Document the deny-path matrix and release-review expectations.
