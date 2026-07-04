## Why

Molten's architecture names Syndicate/SAM-style dataspaces as the default model for ordinary actor traffic, but the current local runtime only exercises a small equality-based dataspace surface and the `syndicate` crate is not yet used directly in runtime code. We need a staged path that turns Syndicate into executable reference semantics without destabilizing Molten's canonical Preserves, policy, evidence, and deterministic replay contracts.

## What Changes

- Add a Syndicate-backed local dataspace reference harness that consumes the same canonical Preserves runtime steps as the existing Molten local dataspace.
- Compare Molten and Syndicate outcomes through canonical Preserves event and receipt refs before replacing production routing.
- Model facets/owners, assertion cleanup, Observe delivery, retractions, capability attenuation, flow-control accounting, and trace collection as evidence-bearing surfaces.
- Bind Syndicate traces and flow-control observations into Molten receipts while keeping them evidence-only and non-authoritative.
- Document that Syndicate remains a semantic/runtime implementation aid, not a wire-protocol or authority substitute.

## Impact

- **Files**: runtime dataspace adapter, test harness, trace/evidence builders, resource/backpressure tests, runtime docs.
- **Testing**: adds parity fixtures, negative authority/attenuation cases, cleanup/retraction tests, and bounded flow-control tests.
