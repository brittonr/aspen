## Why

Molten owns purpose-scoped key backends, entropy, opaque handles, generation-fenced rotation, signing, capability authority, federation membership, Preserves codecs, Iroh transport, and runtime admission. The independent `artifact-auth` repository now publishes a reviewed Molten mapping profile at immutable revision `799459346d5416fbd7b9f55840a7371441b55afa`, but that producer-side profile cannot silently replace Molten-owned identity or authority semantics.

A consumer-owned change is required to review the frozen profile against current fabric cryptographic identity behavior, pin one source, add a pure adapter, and dual-run before any standalone path is admitted. The current Molten path remains authoritative throughout this handoff.

## What Changes

- Review `config/consumers/molten.ncl` and `fixtures/consumers/molten.json` from the immutable standalone revision against current key-generation/currentness behavior.
- Pin Cargo and Nix to one reviewed standalone source only after local source, licensing, and isolation checks pass.
- Add pure adapters for domain, purpose, profile, payload/public-key/verifier-context refs, generation, and currentness.
- Dual-run positive, rotation, and tamper cases with explicit drift classes and bounded rollback.
- Keep key generation/storage/signing, entropy, capability/federation authority, Preserves, Iroh, runtime policy, evidence, and process effects in Molten.

## Impact

- **Planned surfaces**: fabric cryptographic identity cores/adapters, exact Cargo/Nix pins, compatibility fixtures, operator migration documentation, and bounded receipts.
- **No producer-side migration**: this lifecycle package changes no Molten runtime or dependency surface by itself.
- **Claims**: standalone authentication does not grant membership, capability, transport, runtime, deployment, or release authority.
