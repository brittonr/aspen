# Tasks: sealed-repro-redaction-preflight

- [x] [serial] r[molten.testing.sealed_repro_redaction_preflight.policy] Add canonical redaction policy evidence for sealed report repro bundles.
- [x] [serial] r[molten.testing.sealed_repro_redaction_preflight.gate] Add redaction gate evidence bound to embedded report and suite refs.
- [x] [serial] r[molten.testing.sealed_repro_redaction_preflight.scan] Fail closed on sensitive Preserves record markers (`secret`, `confidential`, `credential`, `private`, `encrypted-ref`).
- [x] [serial] r[molten.testing.sealed_repro_redaction_preflight.validation] Recompute redaction policy/gate evidence during sealed bundle parse/gate/verify/unpack.
- [x] [serial] r[molten.testing.sealed_repro_redaction_preflight.unsealed_rejection] Reject unsealed report repro bundles from pass-evidence gates because they lack redaction preflight.
- [x] [parallel] r[molten.testing.sealed_repro_redaction_preflight.tests] Add negative tests for sensitive markers and tampered redaction gate evidence.
- [x] [parallel] r[molten.testing.sealed_repro_redaction_preflight.docs] Update docs and command descriptions.
