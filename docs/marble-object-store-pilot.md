# Marble object-store pilot

r[impl aspen.marble_store.spike] r[impl aspen.marble_store.pinning] r[impl aspen.marble_store.identity] r[impl aspen.marble_store.art_index] r[impl aspen.marble_store.boundary] r[impl aspen.marble_store.verification]

Molten owns the distributed runtime's storage path. This pilot spikes an
alternative physical layer behind the local object storage seam and measures
it against the current Prolly-based path before any storage-path decision.
It is selected by configuration only: the `marble-store` cargo feature mounts
`molten::marble_store`, the default build never compiles it, and the runtime
never selects it unless a caller explicitly opens the backend. The current
path stays the default.

## Spike shape

`molten::marble_store::Backend` stores whole objects in a
[marble](https://github.com/komora-io/marble) heap keyed by allocated
physical handles, and indexes fixed 32-byte BLAKE3 digests to those handles
with the [art](https://github.com/spacejam/art) adaptive radix trie.

- Writes allocate ObjectIds in sharded ranges by object kind (`content`
  payloads, `index` snapshots), batch objects into marble `write_batch`
  calls, and record digest-to-ObjectId mappings in the trie.
- Reads resolve the digest through the trie, then fetch from marble through a
  bounded least-recently-used read cache, re-verifying the BLAKE3 digest
  before serving bytes.
- In-flight batch mappings are served from the backend write cache until
  `write_batch` returns; an interrupted batch discards its cache entries and
  publishes no mappings.
- Maintenance runs on an explicit schedule owned by the backend (every
  configured number of committed batches, or through an explicit call).

## Identity contract

BLAKE3 remains the only content identity. An ObjectId is a physical handle
private to the backend: two identical digests resolve to one logical object,
two different objects never share a digest mapping, and handles never appear
in canonical identity. Physical handles appear in receipts only as private
diagnostic handles.

## Blocking boundary

Marble operations block. The backend confines every blocking operation,
including recovery, to one dedicated bounded executor thread fed by a bounded
command queue; callers queue commands and await bounded replies. Saturating
the queue fails closed.

## Recovery contract

Opening the backend recovers the digest index by re-reading marble's
atomically recovered heap, so the index is a pure function of recovered
state: only batches marble recovered atomically replay, and a batch
interrupted before commit leaves no mappings. A separate-process fixture
kills a writer mid-flight and verifies that every replayed mapping
round-trips its exact bytes and that recovery may exceed the committed
journal by at most one in-flight batch.

## Measurement and decision

`molten::marble_store::compare_paths` runs both paths under one
deterministic synthetic campaign workload and records batch write, point
read, recovery, and space amplification numbers; `keep_or_replace` derives
the recorded decision. `examples/marble-store-measure.rs` prints the
canonical measurement and decision values for one executed run. The recorded
numbers live in
[`evidence/marble-object-store-pilot/decision-record.md`](../evidence/marble-object-store-pilot/decision-record.md).
Timings are recorded diagnostics from the executed run, not deterministic
playback artifacts; claim language stays at `records`, never `proves`.

## Dependency catalog

Both reviewed pins are recorded in
[`docs/marble-object-store-pilot/profile.ncl`](marble-object-store-pilot/profile.ncl)
as transport `crates.io`, plane `implementation`:

- `marble` `=16.0.2` (Apache-2.0 option of MIT/Apache-2.0, komora-io audit
  2026-09-09);
- `art` `=1.0.0` (Apache-2.0 option of MIT OR Apache-2.0).

The dependency catalog lives outside `config/release-dependencies/` because
the release profile binds release git sources only; this pilot uses optional
crates.io spike dependencies behind the `marble-store` feature.

## Non-claims

This pilot changes no dataspace, vat, networking, or replication semantics
and does not replace the storage path. Pilot evidence does not prove:

- sandboxing, hermeticity, or filesystem isolation;
- object meaning, upstream correctness, or suitability of marble or art;
- durability beyond marble's atomically recovered batches;
- performance beyond the recorded numbers of the executed run;
- production readiness or default-path authority.

ChaosControl campaign evidence stays external and recorded by reference in
the decision record; the in-repo fixtures are deterministic stand-ins, not
campaign receipts.
