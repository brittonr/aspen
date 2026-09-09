## Spike backend

- [ ] [serial] Add the optional marble-plus-art backend behind the storage seam, selected by configuration. r[aspen.marble_store.spike]
- [ ] [serial] Confine blocking store operations to a bounded executor and own maintenance scheduling inside the backend. r[aspen.marble_store.spike]
- [ ] [serial] Pin reviewed marble and art revisions and record transport `crates.io`, plane `implementation` in the dependency catalog. r[aspen.marble_store.pinning]
- [ ] [serial] Measure batch write, point read, recovery, and space amplification against the current path under campaign workloads, and record the keep-or-replace decision. r[aspen.marble_store.spike]

## Identity and index

- [ ] [serial] Keep BLAKE3 as the only content identity and keep ObjectIds private physical handles. r[aspen.marble_store.identity]
- [ ] [serial] Index fixed 32-byte digests to ObjectIds with art and fail lookup for absent digests. r[aspen.marble_store.art_index]
- [ ] [serial] Serve in-flight batch mappings from the backend cache until `write_batch` returns and replay only atomically recovered batches after a crash. r[aspen.marble_store.identity]

## Boundary

- [ ] [serial] Document that the spike changes no dataspace, vat, networking, or replication semantics and claims nothing beyond recorded measurements. r[aspen.marble_store.boundary]

## Verification

- [ ] [parallel] Add positive cases for store, resolve, fetch, and crash recovery round-trips. r[aspen.marble_store.verification]
- [ ] [parallel] Add negative cases for absent digests, interrupted batches, and ObjectId exhaustion. r[aspen.marble_store.verification]
- [ ] [parallel] Record ChaosControl campaign references for the spike against the F-series fault set. r[aspen.marble_store.verification]
- [ ] [serial] Run package, workspace, Clippy, Cairn, and Nix checks; append the komora-io reference under `## References` in `README.md`; document non-claims. r[aspen.marble_store.verification]
