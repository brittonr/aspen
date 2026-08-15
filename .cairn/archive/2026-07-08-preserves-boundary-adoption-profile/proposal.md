## Why

Molten already uses Preserves as its canonical envelope spine, but stack-level adoption should be explicit about scope. Preserves should be required at runtime, storage, policy, and evidence boundaries while pure runtime cores continue to consume typed facts and adapters instead of raw Preserves internals.

## What Changes

- Add a Preserves boundary adoption profile for Molten/Aspen.
- Classify boundary surfaces where Preserves canonical bytes are required: node control envelopes, tickets, workflow bundles, receipts, evidence envelopes, and durable refs.
- Require adapters to parse Preserves envelopes into local typed DTOs before core logic runs.
- Add positive and negative fixtures for canonical boundary artifacts, non-canonical bytes, missing schema labels, stale BLAKE3 refs, and accidental raw-Preserves core coupling.

## Impact

- Preserves becomes stronger at Molten's public boundaries without spreading through all internals.
- Valence, Mantle, and Cairn can consume stable artifacts by digest and schema label.
- Runtime semantics stay owned by Molten, not by the serialization format.
