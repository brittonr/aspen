## Why

The highest-leverage Unison-like idea not yet called out as its own Molten slice is a stable package/release view: a named snapshot points to an exact artifact closure plus evidence, while mutable channels remain pointers. This gives operators semantic package stability without adopting UCM packages or Unison namespaces.

Molten release snapshots should bind artifact closures, dependency graphs, policy/provenance/source-gate evidence, compatibility/migration receipts, caveats, and signatures. A channel may point at a snapshot, but the snapshot itself is immutable and exact-ref based.

## What Changes

- Add canonical release/package snapshot artifacts that bind namespace scope, exact artifact set, dependency closure digest, docs/transcripts, policy/evidence refs, compatibility/migration receipts, caveats, and signatures.
- Add closure integrity checks that reject missing artifacts, stale dependency indexes, unexpected refs, wrong hashes, and non-BLAKE3 identity without explicit interop evidence.
- Model release channels as mutable name views pointing to immutable snapshots, not authority or proof of safety.
- Add positive and negative fixtures for snapshot creation, verification, channel update, stale evidence, missing closure members, redaction, and rollback/caveat readback.

## Impact

- **Files**: artifact registry, release evidence, catalog, dependency impact index, upgrade sessions, transcripts, provenance gates, docs.
- **Testing**: positive fixtures for immutable snapshot verification; negative fixtures for tampered members, stale evidence, missing caveats, unauthorized channel moves, and channel-only trust.
- **Security**: release snapshots are evidence bundles. They do not grant authority, deployment, policy trust, provenance, source-gate, retention, transport, or execution by themselves.