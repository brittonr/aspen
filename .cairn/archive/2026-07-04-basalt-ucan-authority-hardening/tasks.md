# Tasks: basalt-ucan-authority-hardening

- [x] [serial] r[molten.capability_token.ucan_verified_authority] Define canonical UCAN verification receipt values that bind compact token refs, proof refs, verification key evidence, caveat decisions, revocation/replay facts, derived grant refs, and request refs.
- [x] [serial] r[molten.runtime_spine.basalt_ucan_verified_authority] Replace bare `ucan_ref` runtime authority checks with a pure Basalt/UCAN authority decision over verified grants and explicit receipt refs.
- [x] [serial] r[molten.testing.capability_context.ucan_proofset_validation] Allow non-empty harness UCAN proofsets only when matching UCAN verification receipts are present and bound to the capability gate.
- [x] [serial] r[molten.testing.capability_context.basalt_enforcement_receipts] Call Basalt enforcement for Basalt-governed harness admission and bind the enforcement receipt into report and gate evidence.
- [x] [parallel] r[molten.evidence.basalt_ucan.authority_receipt_binding] Add gate receipt artifact refs and validation for UCAN verification receipts, Basalt enforcement receipts, proofset refs, derived grant refs, and request refs.
- [x] [parallel] r[molten.capability_token.fixture_not_parallel_authority] Reclassify local capability token fixtures as deterministic fixture inputs or evidence candidates, not a production authority path parallel to verified UCAN/Basalt admission.
- [x] [parallel] r[molten.testing.capability_context.ucan_negative_fixtures] Add positive and negative tests for valid UCAN admission, invalid signature, wrong holder/audience/session/context, expiry, revocation, caveat failure, replay denial, mismatched Basalt policy, local fallback attempts, and tampered receipts.
- [x] [serial] r[molten.runtime_spine.basalt_ucan_trace_denials] Emit runtime and harness denial receipts for missing, stale, or mismatched UCAN/Basalt evidence before side effects commit.
- [x] [serial] r[molten.evidence.basalt_ucan.authority_receipt_binding] Run focused runtime/harness tests and `cairn validate --root .`, or record blockers and the next best check.
