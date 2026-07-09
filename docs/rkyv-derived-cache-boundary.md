# rkyv derived cache boundary

rkyv archives are local, rebuildable sidecars for fast read access. They are not canonical Molten artifacts. Cache keys, cache values, storage value refs, receipts, policy refs, release refs, and evidence refs continue to derive identity from canonical Preserves values and BLAKE3 source refs.

Each rkyv sidecar is described by a canonical Preserves manifest that binds cache purpose, artifact kind, archive profile, producer tool ref/version, canonical source refs and digests, archive byte digest, validation receipt, rebuild capability, retention class, and an explicit `derived-sidecar` identity claim.

Admission is pure before shell IO: the core compares manifest facts with current canonical source refs/digests and observed archive/validation facts, then returns `admit`, `rebuild`, or `deny`. The shell owns byte reads, mmap, bytecheck/rkyv validation, rebuilding, and cache writes.

Typed storage may keep rkyv materializations only as tagged sidecars. Durable stored values, schema conformance, migration planning, and release/evidence checks must read canonical Preserves identity, not rkyv byte layout or process-local memory shape.
