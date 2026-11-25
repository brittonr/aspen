# Authority identity revocation deltas

## ADDED Requirements

### Requirement: Authority admission checks independent principal/key sets and honors revocation effective times
r[molten.authority.independent_currentness_and_revocation_bounds] `admit_authority` and `gatekeeper_resolve_live_ref` MUST evaluate contexts against caller-independent expected principals, minimum epochs, and current key sets. `authority_grant_currentness` MUST reject empty or stale key lists, and `cleanup_for_revocation` MUST enforce `revocation.effective_at <= logical_time` and `revocation.target_kind`.

#### Scenario: Empty key list or mismatched principal denies
- GIVEN an authority context with an empty `keys` list or a `subject_ref` differing from the requested principal
- WHEN `admit_authority` evaluates it
- THEN admission MUST deny with a typed diagnostic.

#### Scenario: Future-dated revocation does not prematurely clean up
- GIVEN a revocation record with `effective_at > logical_time`
- WHEN `cleanup_for_revocation` runs
- THEN active grants MUST NOT be removed before `effective_at`.

### Requirement: Cryptographic identity adapters preserve signer mismatches, avoid reload TOCTOU, and scope revocation per handle
r[molten.authority.crypto_adapter_exact_verification_and_scoped_revocation] The file cryptographic identity adapter MUST NOT overwrite mismatched `signature.metadata.signer_public_ref` before verification, MUST bind signing and transport-secret loading to one atomic file observation, and MUST scope revocation checks to the specific key handle and generation so revoking an old file key does not deny a rotated TPM2 key.

#### Scenario: Mismatched signature metadata denies verification
- GIVEN a signature whose `metadata.signer_public_ref` differs from the verifying key's `public_key_ref`
- WHEN `verify` runs
- THEN `evaluate_verification` MUST observe the mismatch and deny rather than silently overwriting it.

#### Scenario: Revoking old file key preserves rotated TPM2 key
- GIVEN transport identity rotated from a file key to a `Tpm2Sealed` key and the old file key handle is revoked
- WHEN `tpm2::Sealed::unseal` runs
- THEN the new TPM2 key MUST remain usable while the revoked file key handle remains denied.
