## Why

Provenance admission is a trust-state machine over reviewed, reproducible-verified, policy-trusted, denied, and missing evidence. Sensitive operations must not be admitted by weaker trust states or by reproducible records that lack matching build verification.

## What Changes

- Add requirements for provenance trust-state transition and admission proof.
- Require build-record/build-verification binding checks for reproducible-verified provenance.
- Require negative evidence for missing, stale, wrong-artifact, wrong-profile, weak-trust, and mismatched build verification cases.

## Impact

- **Files**: provenance evaluation, node install/run gates, remote sync admission, catalog readback, and production security review tests.
- **Testing**: reviewed pass for scoped low-risk operations, reproducible-verified pass with matching build verification, denial for sensitive weak trust, and denial for stale or mismatched build evidence.
