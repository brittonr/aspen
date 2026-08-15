## Why

Molten already owns canonical chunk manifests, BLAKE3 identities, verification, range reads, partial fetch state, pins, retention gates, and a Redb index. Its current Iroh-shaped exchange copies files through local roots rather than exercising a live streaming content backend.

The fabric needs a content-store adapter boundary that preserves Molten identity while allowing local files, Redb indexes, real Iroh blob exchange, and deterministic simulation to implement the same bounded operations.

## What Changes

- Keep chunking, manifests, content refs, range planning, missing-set calculation, verification, retention eligibility, and receipt payloads as pure primitives.
- Add versioned content-store and content-exchange adapter contracts for streaming put/get, verified range reads, availability, import/export, pin handles, and bounded status.
- Add capability-rooted local storage, Redb index, live Iroh blob, and deterministic simulation profiles behind the same canonical contract.
- Treat backend hashes, tickets, tags, paths, and provider ids as location or protection hints rather than Molten object identity or authority.
- Require bounded streaming and verification before bytes are exposed, indexed as available, pinned, installed, or executed.
- Add shared positive and negative adapter conformance for corruption, truncation, unexpected chunks, unsupported transforms, cancellation, partial fetch, and retention denial.

## Impact

- **Files**: `src/chunk/**`, `src/iroh/**`, local-store capability wrappers, Redb indexes, fabric adapter descriptors, simulation adapters, CLI readback, fixtures, and `cairn/specs/content-addressed-chunk-store/spec.md`.
- **Testing**: live-loopback and simulated adapter parity, streaming/range behavior, partial resume, restart, corruption, capability-root escape, cancellation, retention, and backend-hint tests.
- **Safety**: adapter availability and transport success do not grant import, retention, confidentiality, provenance, execution, or deletion authority.
- **Licensing**: Aspen `main` storage behavior may inform requirements, but code is reused only from explicitly compatible upstream or relicensed sources.
