# Design: Molten artifact-auth operational receipt

## Goal and completion evidence

Complete when an actual `IrohEd25519FileAdapter` signs exact standalone bytes, a canonical bounded receipt is written through `NodeStateNamespaceKind::Receipts`, state is reopened, the key status is re-derived from persisted generation and revocation marker, and replay independently verifies all identities and authority flags.

False completion includes a fixture-only currentness enum, an in-memory round trip, private-key possession as authority, receipt self-hash trust, or standalone admission.

## Portfolio registry

| Family | Mechanism | State | Evidence or blocker |
|---|---|---|---|
| capability-file-state | Reopen persisted key generation and revocation marker | active | Existing adapter denies revoked/stale handles and survives restart. |
| caller-currentness | Trust `VerificationRequest.signer_currentness` | falsified | Caller assertion is not an operational state source. |
| network-revocation | Query external revocation service | blocked | No such product authority exists in this bounded slice. |
| federation-state | Infer currentness from membership/federation | falsified | Membership and key authority are explicitly separate. |

## Boundaries and audit

Pure receipt construction/validation is separate from namespace I/O. Audit receipt traversal, malformed bytes, self-hash exclusion, restart generation drift, rotated/revoked key replay, wrong receipt namespace, wrong key purpose, carrier drift, and false parity. Receipt evidence remains non-authoritative.
