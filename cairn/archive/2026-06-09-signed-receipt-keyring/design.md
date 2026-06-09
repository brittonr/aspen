# Design: signed-receipt-keyring

## Overview

The keyring is a set of immutable Preserves artifacts stored in the existing local evidence ledger:

- `signed-receipt-key-v1` records bind key id, signer, trust root, fixture verification key material, generation, predecessor ref, and evidence-only checks.
- `signed-receipt-key-revocation-v1` records bind a revoked key ref, its identity fields, reason, optional superseding key ref, and evidence-only checks.

Verification loads key and revocation records from the ledger, filters by signed envelope signer/trust-root and optional key id/ref, requires exactly one current unrevoked key, then verifies the signed envelope using that key. Revocations are immutable evidence: a key record remains unchanged, but any matching revocation record makes the key ineligible for future keyring verification.

## CLI

Top-level receipt keyring commands:

- `molten receipts key import --ledger LEDGER --key-id ID --signer SIGNER --trust-root ROOT --key KEY`
- `molten receipts key list --ledger LEDGER`
- `molten receipts key show REF --ledger LEDGER`
- `molten receipts key revoke REF --ledger LEDGER --reason REASON`
- `molten receipts key rotate REF --ledger LEDGER --new-key-id ID --new-key KEY`

Signed receipt verification accepts optional keyring inputs:

- `molten receipts verify-signed SIGNED --key-ledger LEDGER [--key-id ID|--key-ref REF] ...`
- `molten test receipt verify SIGNED --key-ledger LEDGER [--key-id ID|--key-ref REF] ...`

Release bundle verification accepts signed-member keyring inputs:

- `molten dogfood release-bundle-verify ... --require-signed-members --signed-key-ledger LEDGER [--signed-key-id ID|--signed-key-ref REF] ...`

## Evidence Boundary

Keyring records and revocation records are evidence for signature verification only. They do not grant authority to perform privileged work and do not replace source, provenance, policy, resource, retention, transport, or destructive-operation gates.

## Failure Modes

Verification emits fail-closed diagnostics when:

- no matching key exists;
- more than one current key matches without a disambiguating key id/ref;
- the selected key is not current;
- the selected key is revoked;
- signer, purpose, trust root, subject ref, key id/ref, or signature material mismatches;
- signed bundle members are missing or bound to subjects outside the bundle.
