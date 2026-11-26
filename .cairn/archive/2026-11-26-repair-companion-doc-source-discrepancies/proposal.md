## Why

The technical companion under `docs/technical/` records seven implementation-vs-spec discrepancies that no active Cairn change tracks. The defect-tracking rule introduced by the `align-formal-models-architecture-and-governance-toolchains` change requires each one to be a tracked defect, not just a companion-doc disclaimer:

1. **Overlap signing (`docs/technical/cryptography/key-rotation-and-generation-fences.md`):** the governing fabric identity document makes rotation overlap verification-only. `crates/molten-core/src/fabric_crypto_identity/rotation.rs` has a `SignAndVerify` overlap mode, and both overlap modes map to `KeyCurrentness::Overlap`, which `permits_signing` accepts in `plan_sign`.
2. **Retry deadline domain (`docs/technical/time/deadline-retry-and-lease-limits.md`):** the time contract denies wall-clock deadlines, but `plan_retry` builds a `Deadline` with `checked_add_duration` without calling the deadline-domain validator.
3. **Entropy capability binding (`docs/technical/time/purpose-bound-entropy.md`):** the governing prose places capability-bound stream metadata in entropy evidence, but `EntropyEvidenceMetadata` and the canonical entropy event carry no `capability_ref`.
4. **Descriptor-bridge error class (`docs/technical/engineering/test-workspace-lifetime-and-authority.md`):** the test-workspace guide says hosts without the descriptor bridge return `Unsupported`. On a Unix host without usable `/proc/self/fd`, the helper propagates the raw `read_link` error instead.
5. **Snapshot receipt check label (`docs/technical/vats/snapshot-and-restore-authority.md`):** the snapshot evaluator emits `readable-assertions-subset-claimed`, but the validator checks readable assertions against admitted references, not claimed ones.
6. **Key status handle reference (`docs/technical/cryptography/entropy-and-key-storage-admission.md`):** the governing identity status prose includes an opaque handle reference, but the canonical public status record in `src/fabric_crypto_identity/parts/canonical/p001/body.rs` has no handle-reference field.
7. **Capability context default (`docs/technical/capabilities/capability-context-admission.md`):** the architecture describes deny-by-default grants, but `CapabilityContext::default()` in `src/runtime/admission/mod.rs` calls `allow_all()`.

## What Changes

- Restrict rotation overlap to verification only: remove or deny `SignAndVerify` overlap and make `Overlap` currentness unable to satisfy `permits_signing`.
- Validate the deadline domain in `plan_retry` before returning a `Deadline`, so a wall-clock base is denied at construction.
- Bind the entropy capability reference into canonical entropy evidence, or narrow the governing prose to what the event actually binds, as decided in design.
- Classify a missing or unusable `/proc/self/fd` descriptor bridge as `Unsupported` on every host.
- Rename the snapshot receipt check to match the predicate it evaluates (`readable-assertions-subset-admitted`), with a versioned receipt label.
- Add the opaque key handle reference to the canonical public key status record under a versioned status label.
- Make `CapabilityContext::default()` deny everything; allow-all requires the explicit `allow_all()` constructor.
- Update each companion article to drop its discrepancy note once the fix lands.

## Impact

- **Files**: `crates/molten-core/src/fabric_crypto_identity/{rotation,model}.rs`, `src/fabric_crypto_identity/parts/canonical/p001/body.rs`, `crates/molten-core/src/fabric_time/` retry planning and entropy evidence, `src/test/` workspace descriptor helper, `src/runtime/predicates/parts/mod/p003/body.rs`, `src/runtime/admission/mod.rs`, and the seven companion articles.
- **Testing**: positive and negative unit tests per requirement (overlap signing denied, wall-clock retry denied, entropy evidence binds the capability ref, missing bridge yields `Unsupported`, snapshot receipt label matches its predicate), plus `cargo clippy`, Octet, and strict Cairn validation.

## Out of Scope

- Discrepancies already tracked by other changes (envelope, DAG exchange, transport replay, context overrides, proof composition, static audits, promotion, merge, local startup, and core lowercase BLAKE3 refs).
