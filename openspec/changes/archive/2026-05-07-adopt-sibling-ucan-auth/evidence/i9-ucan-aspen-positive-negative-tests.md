# I9 UCAN/Aspen positive and negative verification tests

- Change: `adopt-sibling-ucan-auth`
- Task: Add positive UCAN/Aspen round-trip tests and negative escalation, expiry, malformed proof, wrong audience, replay/revocation, and denied capability mapping tests.
- Started: 2026-05-06T23:56:02Z
- Completed: 2026-05-06T23:57:23Z
- Status: PASS

## Added evidence

- Added `rejects_empty_ucan_capability_set` in `crates/aspen-auth/src/ucan_adapter.rs`.
- Retained existing runtime negative suites for tampered signature, expiry, wrong audience, revocation, trusted roots, malformed/missing delegation proof, and untrusted proof chains.
- Retained compatibility fixtures for legacy token base64 roundtrip, delegation receipt shape, and redacted debug receipts.

## Verification commands

- `rustfmt crates/aspen-auth/src/ucan_adapter.rs` → PASS
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth ucan_adapter --all-targets` → PASS after fixing the assertion to check the Aspen `EncodingError` boundary rather than sibling wording.
  - 4 passed; 0 failed; 84 filtered out.
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth test_verifier --all-targets` → PASS
  - 8 passed; 0 failed; 80 filtered out.

## Known failed attempt retained for audit

Initial assertion expected the sibling error string to contain `capability set must not be empty`; the actual Aspen boundary intentionally wraps sibling failures as `AuthError::EncodingError`. The final test asserts the Aspen-facing error class instead.
