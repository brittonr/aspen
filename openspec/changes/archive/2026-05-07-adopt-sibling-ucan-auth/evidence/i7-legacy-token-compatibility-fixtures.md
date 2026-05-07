# I7 legacy Aspen token compatibility fixtures

- Change: `adopt-sibling-ucan-auth`
- Task: Add compatibility fixtures for existing Aspen token generation/inspection/delegation behavior or documented migration receipts for intentional format changes.
- Started: 2026-05-06T23:49:32Z
- Completed: 2026-05-06T23:50:40Z
- Status: PASS

## Fixtures added

Added focused compatibility fixtures in `crates/aspen-auth/src/tests.rs`:

- `adopt_sibling_ucan_compat_fixture_preserves_legacy_base64_roundtrip`
  - Builds a legacy Aspen `CapabilityToken` with `TokenBuilder`.
  - Encodes/decodes through existing postcard + URL-safe base64.
  - Confirms version, issuer, capabilities, facts, and signature survive the round trip.
- `adopt_sibling_ucan_compat_fixture_preserves_delegation_receipt_shape`
  - Builds a delegating root token and attenuated child token.
  - Confirms child depth increments and `proof` remains the parent token hash.
- `adopt_sibling_ucan_compat_fixture_redacts_debug_receipts`
  - Confirms debug receipts still report redacted signature material and do not expose nonce bytes.

## Verification

- `rustfmt crates/aspen-auth/src/tests.rs` → PASS
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth adopt_sibling_ucan_compat_fixture --all-targets` → PASS
  - 3 passed; 0 failed; 84 filtered out.

## Boundary conclusion

No intentional token-format migration happened in this slice. Existing Aspen token generation, inspection/debug redaction, and delegation receipt behavior remain preserved while the sibling UCAN adapter is staged behind separate APIs.
