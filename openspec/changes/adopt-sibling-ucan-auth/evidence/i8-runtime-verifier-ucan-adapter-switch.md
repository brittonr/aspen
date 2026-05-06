# I8 runtime verifier UCAN adapter switch

- Change: `adopt-sibling-ucan-auth`
- Task: Switch runtime verification/admission paths to the UCAN-backed verifier only after compatibility and negative evidence exists.
- Started: 2026-05-06T23:50:40Z
- Completed: 2026-05-06T23:53:01Z
- Status: PASS

## Implementation

Updated `crates/aspen-auth/src/verifier.rs` so both runtime token verification paths validate every presented token capability through the sibling-backed adapter:

- `TokenVerifier::verify_internal`
- `TokenVerifier::verify_with_chain_internal`

After signature verification and before time/audience/revocation/delegation-chain checks, the verifier now calls `ucan_adapter::capabilities_to_ucan_set(&token.capabilities)`. This preserves the legacy Aspen `CapabilityToken` wire format while requiring runtime-admitted capabilities to project into sibling-validated UCAN capability documents.

The legacy Aspen operation authorization check remains in place for this slice. That preserves existing RPC/CLI semantics while staging UCAN resource/ability validation on the token-admission boundary.

## Verification

- `rustfmt crates/aspen-auth/src/verifier.rs` → PASS
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth test_verifier --all-targets` → PASS
  - 8 passed; 0 failed; 79 filtered out.

## Negative evidence already retained before switch

- `test_verifier_rejects_tampered_signature`
- `test_verifier_rejects_expired_token`
- `test_verifier_checks_audience`
- `test_verifier_revocation`
- `test_verifier_trusted_roots`
- `test_delegated_token_requires_parent_for_chain_verification`
- `test_verify_with_chain_method`
- `test_chain_verification_rejects_untrusted_root_in_chain`

## Boundary conclusion

The runtime admission path is now UCAN-adapter-backed for capability document validation. Full UCAN compact-token wire-format replacement remains intentionally out of scope for this OpenSpec slice because compatibility fixtures currently preserve the Aspen token format.
