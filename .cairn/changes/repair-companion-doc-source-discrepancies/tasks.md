## Phase 1: Implementation

- [ ] [parallel] Make rotation overlap verification-only in `crates/molten-core/src/fabric_crypto_identity/{rotation,model}.rs`, with tests showing overlap signing is denied and overlap verification still passes. r[molten.crypto_identity.overlap_verification_only]
- [ ] [parallel] Validate the deadline domain in `plan_retry` and add tests showing a wall-clock base is denied and a monotonic base is accepted. r[molten.fabric_time.retry_deadline_domain]
- [ ] [parallel] Bind `capability_ref` into versioned entropy evidence and reject mismatched references on replay, with positive and negative tests. r[molten.fabric_time.entropy_capability_evidence]
- [ ] [parallel] Classify a missing `/proc/self/fd` descriptor bridge as `Unsupported`, with tests for both the missing-bridge and the present-bridge cases. r[molten.testing.descriptor_bridge_unsupported]
- [ ] [parallel] Version the snapshot receipt check label to `readable-assertions-subset-admitted`, with tests showing the label matches the evaluated predicate. r[molten.runtime_spine.snapshot_receipt_check_label]
- [ ] [parallel] Add the opaque handle reference to the versioned canonical public key status record, with tests showing the field round-trips and carries no key material. r[molten.crypto_identity.status_handle_reference]
- [ ] [parallel] Make `CapabilityContext::default()` deny everything, migrate each `Default` caller to explicit grants or `allow_all()`, and add tests showing the default denies and an explicit grant admits. r[molten.runtime_spine.capability_context_deny_by_default]

## Phase 2: Validation

- [ ] [serial] Remove the seven discrepancy notes from `docs/technical/` and the README list, then run focused tests, Clippy, Octet, documentation-link checks, and strict Cairn validation. r[molten.crypto_identity.overlap_verification_only] r[molten.crypto_identity.status_handle_reference] r[molten.fabric_time.retry_deadline_domain] r[molten.fabric_time.entropy_capability_evidence] r[molten.testing.descriptor_bridge_unsupported] r[molten.runtime_spine.snapshot_receipt_check_label] r[molten.runtime_spine.capability_context_deny_by_default]
