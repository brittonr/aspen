# Tasks: nickel-array-invariants

## Phase 1: Prelude helpers

- [x] [serial] r[molten.nickel_array_invariants.shared_array_helpers] Add shared Nickel helpers for bounded arrays, unique arrays, non-empty unique arrays, unique ref arrays, and required-member arrays.
- [x] [parallel] r[molten.nickel_array_invariants.helper_diagnostics] Add named predicates or fixtures so failures identify the intended array invariant.

## Phase 2: Contract adoption

- [x] [serial] r[molten.nickel_array_invariants.production_peer_multinode] Apply array helpers to production profile, peer profile, and multinode scenario contracts.
- [x] [serial] r[molten.nickel_array_invariants.plugin_arrays] Apply array helpers to plugin extension contracts and grants for lifecycle callbacks, descriptor identities, refs, evidence, and grants.
- [x] [parallel] r[molten.nickel_array_invariants.policy_arrays] Apply array helpers to Cairn policy contract arrays where ids or tokens must be unique.

## Phase 3: Fixtures and validation

- [x] [parallel] r[molten.nickel_array_invariants.negative_arrays] Add negative fixtures for duplicates, oversized arrays, missing required members, and contradictory list semantics.
- [x] [serial] r[molten.nickel_array_invariants.shared_array_helpers] Run contract export drift gate, production profile fixture checks, and `nix run path:$PWD#cairn -- validate --root .`.
