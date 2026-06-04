## Phase 1: Admission evidence parser

- [x] [serial] r[molten.testing.admission_evidence.records] Define parser/renderer helpers for canonical admission decision records and derived admission requests.
- [x] [serial] r[molten.testing.admission_evidence.step_binding] Bind each admission request to the corresponding suite step and canonical step hash.
- [x] [serial] r[molten.testing.admission_evidence.policy_recompute] Recompute recorded decisions from the embedded static policy fixture instead of trusting report data.

## Phase 2: Fail-closed report validation

- [x] [serial] r[molten.testing.admission_evidence.mandatory_decision] Make report validation reject observations with missing, duplicated, malformed, or misplaced admission decisions.
- [x] [serial] r[molten.testing.admission_evidence.deny_rollback] Reject denied non-effect turns that commit messages, assertions, retractions, or other semantic state changes after a denial.
- [x] [serial] r[molten.testing.admission_evidence.denied_effect_suppression] Reject denied clock/random/effect steps that contain effect request or effect response records.
- [x] [parallel] r[molten.testing.admission_evidence.failure_artifacts] Emit canonical validation/replay failure diagnostics for admission evidence failures.

## Phase 3: Replay and gate receipts

- [x] [serial] r[molten.testing.admission_evidence.policy_divergence] Classify replay mismatches at admission decision boundaries as `policy-decision` divergence before downstream trace or state drift.
- [x] [serial] r[molten.testing.admission_evidence.gate_checks] Add `admission-policy`, `admission-decisions`, `deny-rollback`, and `denied-effect-suppression` to pass-evidence gate receipts.
- [x] [parallel] r[molten.testing.admission_evidence.repro_bundle] Ensure repro bundles preserve policy fixtures, admission events, and failure diagnostics needed to reproduce admission evidence failures.

## Phase 4: Tests and future policy boundary

- [x] [serial] r[molten.testing.admission_evidence.negative_tests] Add negative suites for missing admission event, tampered allow/deny, denied turn with committed action, and denied effect with response.
- [x] [serial] r[molten.testing.admission_evidence.receipt_tests] Add gate receipt tests proving admission checks are listed and parsed.
- [x] [parallel] r[molten.testing.admission_evidence.nickel_path] Document and structure the static Preserves policy fixture so Nickel contracts and Basalt/UCAN context can replace or augment it without removing fail-closed validation.
