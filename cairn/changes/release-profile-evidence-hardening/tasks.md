# Tasks: release-profile-evidence-hardening

## Phase 1: Profile tiers and placeholder denial

- [ ] [serial] r[molten.prod_ops.release_profile.tiers] Define development, pilot, and release deployment profile semantics in the production profile contract or a release-profile wrapper.
- [ ] [serial] r[molten.prod_ops.release_profile.no_placeholder_refs] Add release-tier placeholder ref denial for all-zero refs, repeated-character dummy refs, and declared fixture placeholders.
- [ ] [parallel] r[molten.prod_ops.release_profile.freshness] Add freshness/readback checks that bind release profile exports to current source-gate, Octet, Cairn, policy, and generated JSON refs.

## Phase 2: Stack provenance release evidence

- [ ] [parallel] r[molten.evidence.stack_provenance.release_required] Require release-tier stack provenance evidence while preserving the non-authority boundary.
- [ ] [parallel] r[molten.evidence.stack_provenance.non_placeholder_hashes] Reject placeholder accepted Valence policy hashes in release-scoped Cairn policy or release profile config.

## Phase 3: Fixtures, docs, and validation

- [ ] [serial] r[molten.prod_ops.release_profile.fixtures] Add positive development/pilot/release fixtures and negative zero-ref, dummy-ref, stale-ref, optional-provenance, and missing-evidence fixtures.
- [ ] [serial] r[molten.evidence.stack_provenance.release_required] Update operator runbooks to describe profile tiers, stack provenance evidence-only status, and release fixture regeneration commands.
- [ ] [serial] r[molten.prod_ops.release_profile.fixtures] Run focused production profile/release evidence tests, contract export drift gate, and Cairn validation/gates.
