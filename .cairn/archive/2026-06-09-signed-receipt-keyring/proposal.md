# signed-receipt-keyring

## Summary

Add a ledger-backed keyring for signed evidence receipts so release and dogfood verification can bind signatures to auditable key records, currentness, revocations, and rotations instead of only ad-hoc CLI key strings.

## Motivation

Signed release evidence receipts already bind canonical subjects, signer identity, purpose, trust roots, and parent refs. Operators also need durable evidence for which signing keys were admitted, which keys are current, and which keys were revoked or rotated. The keyring must remain evidence-only: it improves verification diagnostics and release review reproducibility, but it does not grant subsystem authority.

## Scope

- Add canonical signed receipt key and key revocation records.
- Add top-level `molten receipts key import/list/show/revoke/rotate` CLI commands backed by the local evidence ledger.
- Allow signed receipt verification and release bundle signed-member checks to resolve keys from a ledger keyring.
- Fail closed for missing, ambiguous, stale, revoked, wrong-signer, wrong-purpose, wrong-trust-root, and wrong-subject keyring verification.
- Keep keyring artifacts evidence-only.

## Non-Goals

- Production cryptographic key management, HSM integration, or network trust distribution.
- Granting authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust from signatures or key records.
