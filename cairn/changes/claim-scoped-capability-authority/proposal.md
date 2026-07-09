## Why

Molten can already express external authority with capability tokens, UCAN verification receipts, Basalt enforcement receipts, peer sessions, and local policy/resource gates. What is missing is a standard claim vocabulary for the operator intent: "accept this external peer or cluster as an authority for this narrow class of claims." Without that vocabulary, callers may invent ad hoc trust records, overuse peer sessions, or treat artifact discovery as admission.

This change makes claim-scoped external authority first-class while preserving the existing rule: current authority comes only from admitted capability proofsets and UCAN/Basalt evidence, never from transport identity, peer friendship, ledger possession, or a broad trusted-peer flag.

## What Changes

- Define canonical claim-domain and subject-selector records that can name arbitrary subject spaces, not only BLAKE3 refs.
- Define canonical `authority-claim-v1` evidence records for external statements about subjects and classes.
- Define `authority-claim-admission-v1` receipts that bind a claim to the existing capability admission, UCAN verification, Basalt enforcement, local policy/resource, freshness, and revocation evidence.
- Standardize capability vocabulary for claim authority, such as `claim:attest` with scoped claim kinds and selector-backed resources.
- Ensure peer/session, handoff, transport, registry, and catalog evidence remain non-authoritative unless the existing capability/UCAN/Basalt path passes for the exact claim.

## Impact

- **Files**: capability token helpers, claim authority records, artifact registry/catalog classification, peer diagnostics, CLI/readback UX, docs, and positive/negative tests.
- **Testing**: positive admitted external claim fixtures and negative missing-proof, wrong-holder, wrong-scope, revoked, stale, transport-only, registry-only, and over-broad wildcard fixtures.
- **Security**: adds no parallel trust system. External claim authority is a use of current capability tokens and UCAN/Basalt receipts with a clearer vocabulary and audit trail.
