## Context

The chunk store is both a core identity boundary and an adapter-heavy subsystem. It owns canonical manifests and refs, but also local files, index updates, Iroh-shaped exchange, retention pins, and lineage receipts.

## Design

### Proposed chunk modules

- `model`: chunk refs, manifests, roots, metadata inputs.
- `codec`: canonical manifest/ref construction and parsing.
- `verify`: pure manifest and chunk consistency decisions over in-memory bytes/refs.
- `fs_store`: local chunk/manifest filesystem adapter.
- `index`: Redb or store-port index adapter.
- `exchange`: Iroh ticket/blob exchange shell.
- `retention`: pin/GC integration through retention plans.
- `lineage`: chain evidence and receipt constructors.
- `shell`: orchestration for commands and fixtures.

### Identity preservation

Refactors must preserve existing manifest bytes, chunk refs, lineage refs, and parser decisions unless a separate schema change owns the version break.

### Retention boundary

Chunk GC or deletion must consume admitted retention plans; the chunk store must not bypass retention admission because bytes are locally present.

## Non-goals

- Do not change the chunk manifest schema.
- Do not replace Iroh exchange semantics.
- Do not make local file presence evidence of authority or retention clearance.
