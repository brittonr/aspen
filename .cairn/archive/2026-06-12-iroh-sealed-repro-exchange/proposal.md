# Change: iroh-sealed-repro-exchange

## Why

Sealed repro bundles are useful locally, but Molten/Aspen-style operations need a way to exchange pass evidence and diagnostics between peers without inventing a parallel transport model. Iroh blobs can move immutable bundle content while Molten preserves canonical refs, receipts, policy gates, and redaction guarantees.

## What

- Publish sealed repro bundles and related receipt chains through Iroh blobs using canonical bundle refs as content identities.
- Fetch bundles by ticket/ref and verify before unpacking or importing into the local evidence ledger.
- Bind exchange receipts to local node identity, remote peer identity, blob ticket/ref, bundle ref, and verification result.
- Preserve redaction/confidentiality behavior: private material is not fetched, revealed, or unpacked without explicit reveal authority.
- Keep gossip/docs/discovery as optional follow-up; first slice can use explicit blob tickets.

## Impact

This introduces distributed repro exchange while keeping trust decisions local and evidence-based. Transport success alone never implies gate acceptance.
