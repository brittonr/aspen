# Tasks: production-profile-cross-field-invariants

- [x] [serial] r[molten.prod_ops.profile_invariants.required_evidence] Add profile-level export checks for non-empty source-gate evidence and required production adapter coverage.
- [x] [serial] r[molten.prod_ops.profile_invariants.layout_distinct] Add profile-level checks that state layout subdirectories are distinct and cannot alias one logical store to another.
- [x] [serial] r[molten.prod_ops.profile_invariants.resource_relationships] Add profile-level checks for coherent resource-limit relationships.
- [x] [parallel] r[molten.prod_ops.profile_invariants.required_evidence] Add negative coverage for missing source-gate inputs and missing core adapters.
- [x] [parallel] r[molten.prod_ops.profile_invariants.layout_distinct] Add negative coverage for layout directory collisions.
- [x] [parallel] r[molten.prod_ops.profile_invariants.resource_relationships] Add negative coverage for incoherent receipt, store, delivery, and recovery limits.
