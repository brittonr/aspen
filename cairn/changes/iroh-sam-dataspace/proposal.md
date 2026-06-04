## Why

Molten needs an early distributed actor milestone that proves the selected runtime shape is still coherent: Syndicate/SAM-style dataspace semantics, canonical Preserves communication/evidence, and Iroh as the first remote substrate. Existing work covers local deterministic harnesses, sealed bundle exchange, chunk exchange, federation hints, and job sync, but ordinary actor/dataspace traffic still lacks a specific remote rail.

## What Changes

- Define canonical Preserves records for remote dataspace envelopes carrying actor messages, assertions, retractions, and observe requests.
- Bind envelope identity to canonical Preserves bytes and content refs; large payload bytes remain out-of-band through blob/chunk refs.
- Add Iroh gossip transport receipts for envelope-sized traffic and Iroh blob/chunk validation for referenced payloads.
- Route delivered remote envelopes into the same local SAM-style runtime semantics used by local actors: assertions, retractions, observes, messages, and turn evidence.
- Require peer bootstrap, authority/capability, resource, policy, and transport evidence before remote delivery can become pass evidence.
- Make replay deterministic by recording the transport delivery log, or mark unrecorded live transport runs as non-replayable and ineligible for deterministic gates.

## Impact

This is the first vertical slice where two Molten peers can exchange ordinary actor/dataspace traffic over the intended remote substrate without letting Iroh define semantics. Iroh only transports canonical Molten envelopes and content blobs; Molten/SAM semantics, Preserves identity, and admission/evidence gates remain authoritative.
