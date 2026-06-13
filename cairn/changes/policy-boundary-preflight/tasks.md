## Phase 1: Policy preflight evidence

- [x] [serial] r[molten.testing.policy_boundary.policy_snapshot] Canonicalize the embedded static policy fixture into a normalized policy snapshot ref, including the default allow-all policy.
- [x] [serial] r[molten.testing.policy_boundary.preflight_receipt] Emit canonical `<policy-gate-v1 ...>` evidence before observations in harness reports.
- [x] [serial] r[molten.testing.policy_boundary.preflight_receipt] Run policy preflight before runtime turns or ambient effect requests can execute.

## Phase 2: Fail-closed validation

- [x] [serial] r[molten.testing.policy_boundary.preflight_receipt.missing] Make report validation reject missing, malformed, or unsupported policy gate evidence.
- [x] [serial] r[molten.testing.policy_boundary.policy_snapshot] Verify policy gate refs against the embedded suite's normalized policy snapshot.
- [x] [serial] r[molten.testing.policy_boundary.steel_review] Reject unreviewed Steel/dynamic predicate records in local harness policy fixtures.

## Phase 3: Gate receipts and tests

- [x] [serial] r[molten.testing.policy_boundary.gate_receipts] Add policy preflight, Nickel static policy, Basalt policy gate, and Steel predicate review checks plus policy refs to pass-evidence gate receipts.
- [x] [serial] r[molten.testing.policy_boundary.preflight_receipt.missing] Add negative tests for missing policy gate evidence, stale policy refs, and unreviewed Steel predicate records.
- [x] [parallel] r[molten.testing.policy_boundary.nickel_static] Document the future Nickel/Basalt/Steel replacement seam without weakening the current fail-closed boundary.
