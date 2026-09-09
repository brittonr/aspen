## Goals

- Measure marble plus art against the current storage path under campaign workloads.
- Preserve BLAKE3 content identity exactly.
- Keep the spike behind the storage seam and reversible.

## Spike shape

The storage seam gains one optional backend:

- writes allocate ObjectIds in sharded ranges by object kind, batch objects into marble `write_batch` calls, and record digest-to-ObjectId mappings in `art`;
- reads resolve the digest through `art`, then fetch from marble;
- a bounded read cache sits in front of marble, because marble reads always touch disk;
- maintenance runs on an explicit schedule owned by the backend.

## Identity contract

BLAKE3 remains the only content identity. An ObjectId is a physical handle private to the backend. Two identical digests MUST resolve to one logical object; two different objects MUST never share a digest mapping.

## Measurement plan

Compare against the current path on: batch write latency, point read latency, crash recovery correctness and time, space amplification after maintenance, and fault-campaign outcomes for the F-series delivery and shutdown faults. Record numbers in the decision record; keep the claim language at `records`, not `proves`.

## Verification

Positive coverage: store, resolve, and fetch round-trips; recovery after simulated crash replays atomically recovered batches only.

Negative coverage: missing digest fails lookup; interrupted batch leaves no mappings; ObjectId exhaustion fails closed.

Boundary coverage: no runtime claim beyond recorded measurements; no default-path change; ChaosControl campaign evidence stays external and recorded by reference.
