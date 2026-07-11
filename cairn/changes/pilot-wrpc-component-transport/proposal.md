## Why

wRPC offers WIT-shaped, transport-agnostic RPC for WebAssembly components and out-of-tree runtime plugins. Molten already owns Iroh transport sessions, canonical Preserves envelopes, peer admission, authority, replay logs, and delivery evidence. Adopting wRPC as a replacement would split wire identity and authority semantics, but a bounded adapter pilot can determine whether WIT-native remote component calls improve interoperability without weakening those boundaries.

The pilot also needs a clear telemetry boundary: OpenTelemetry-WASI may expose component diagnostics, but telemetry must not become canonical execution evidence or leak sensitive payloads.

## What Changes

- Add an opt-in wRPC adapter profile for a single versioned Molten component world and pinned wRPC compatibility cohort.
- Carry wRPC calls over one admitted transport-session adapter while preserving Iroh peer/session evidence and canonical Preserves request/result envelope identities.
- Bind each call to explicit policy, Basalt/UCAN authority, resource, delivery, and replay context rather than treating WIT or transport connectivity as authority.
- Exclude evolving WIT stream/future behavior from the first pilot unless separately versioned and admitted.
- Emit bridge/transcript receipts that bind WIT function identity, component-value wire bytes, Preserves envelope refs, transport-session refs, and result classification.
- Permit optional OpenTelemetry-WASI export only as redacted diagnostic telemetry with explicit non-evidence labels.

## Impact

- **Surfaces**: component RPC adapters, Iroh transport sessions, WIT packages, Preserves bridge DTOs, authority/resource gates, replay transcripts, optional telemetry, and pilot fixtures.
- **Dependencies**: this pilot depends on the shared Wasm component runtime profile and the active fabric transport-session boundary.
- **Scope**: the pilot does not replace Iroh, Preserves, Molten delivery semantics, local component calls, or existing remote dataspace protocols.
- **Claims**: successful loopback proves only the pinned adapter fixture; it is not protocol stability, production readiness, transport security, or semantic-equivalence proof.
