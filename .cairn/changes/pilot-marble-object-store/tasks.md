## Spike backend

- [x] [serial] Add the optional marble-plus-art backend behind the storage seam, selected by configuration. r[aspen.marble_store.spike]
- [x] [serial] Confine blocking store operations to a bounded executor and own maintenance scheduling inside the backend. r[aspen.marble_store.spike]
- [x] [serial] Pin reviewed marble and art revisions and record transport `crates.io`, plane `implementation` in the dependency catalog. r[aspen.marble_store.pinning]
- [x] [serial] Measure batch write, point read, recovery, and space amplification against the current path under campaign workloads, and record the keep-or-replace decision. r[aspen.marble_store.spike]

## Identity and index

- [x] [serial] Keep BLAKE3 as the only content identity and keep ObjectIds private physical handles. r[aspen.marble_store.identity]
- [x] [serial] Index fixed 32-byte digests to ObjectIds with art and fail lookup for absent digests. r[aspen.marble_store.art_index]
- [x] [serial] Serve in-flight batch mappings from the backend cache until `write_batch` returns and replay only atomically recovered batches after a crash. r[aspen.marble_store.identity]

## Boundary

- [x] [serial] Document that the spike changes no dataspace, vat, networking, or replication semantics and claims nothing beyond recorded measurements. r[aspen.marble_store.boundary]

## Verification

- [x] [parallel] Add positive cases for store, resolve, fetch, and crash recovery round-trips. r[aspen.marble_store.verification]
- [x] [parallel] Add negative cases for absent digests, interrupted batches, and ObjectId exhaustion. r[aspen.marble_store.verification]
- [ ] [parallel] Record ChaosControl campaign references for the spike against the F-series fault set. r[aspen.marble_store.verification] Blocked: the F-series findings are source-level logic bugs, not storage faults, and no ChaosControl campaign exists for this backend (evidence/2026-10-22-marble-spike.md).
- [x] [serial] Run package, workspace, Clippy, Cairn, and Nix checks; append the komora-io reference under `## References` in `README.md`; document non-claims. r[aspen.marble_store.verification]
