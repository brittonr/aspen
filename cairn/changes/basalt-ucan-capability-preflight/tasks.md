# Tasks: basalt-ucan-capability-preflight

- [x] [serial] r[molten.testing.basalt_ucan_capability_preflight.authority_receipt] Bind capability gates to Basalt authority contract envelopes and preflight receipts.
- [x] [serial] r[molten.testing.basalt_ucan_capability_preflight.proofset_binding] Include explicit UCAN proofset refs and fail closed on non-empty proofsets until validation exists.
- [x] [serial] r[molten.testing.basalt_ucan_capability_preflight.grant_ref_binding] Bind capability gates and admission authority evidence to canonical grant refs.
- [x] [serial] r[molten.testing.basalt_ucan_capability_preflight.validation] Recompute capability preflight evidence from embedded suites and reject stale or tampered gates.
- [x] [serial] r[molten.testing.basalt_ucan_capability_preflight.gate_receipts] Add authority receipt, proofset binding, and grant-ref binding checks/refs to pass-evidence receipts.
- [x] [parallel] r[molten.testing.basalt_ucan_capability_preflight.negative_tests] Add negative coverage for missing/tampered authority receipts, non-empty UCAN proofsets, and tampered grant refs.
- [x] [parallel] r[molten.testing.basalt_ucan_capability_preflight.docs] Update docs and examples of evidence rails.
