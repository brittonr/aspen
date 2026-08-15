# Tasks: nickel-contract-prelude-library

- [x] [serial] r[molten.project.nickel_contract_prelude.shared_helpers] Create a shared Nickel contract prelude with pure helpers for common scalar, array, metadata, allowed-value, and uniqueness domains.
- [x] [serial] r[molten.project.nickel_contract_prelude.shared_helpers] Migrate production profile, multinode scenario, peer profile, plugin extension, and Cairn policy contracts to import shared helpers where behavior is equivalent.
- [x] [parallel] r[molten.project.nickel_contract_prelude.authoring_boundary] Document the authoring-time-only boundary and confirm runtime Rust continues to consume checked exports instead of evaluating Nickel.
- [x] [parallel] r[molten.project.nickel_contract_prelude.shared_helpers] Add fixture coverage proving migrated modules preserve valid exported values and still reject malformed common domains.
- [x] [serial] r[molten.project.nickel_contract_prelude.shared_helpers] Run focused Nickel fixture validation and `cairn validate --root .`, or record the blocker and next best check.
