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

Compare against the current path on batch write latency, point read latency, crash recovery correctness and time, and space amplification after maintenance. The F-series source findings are not a physical-storage campaign. Record actual numbers in the decision record; keep the claim language at `records`, not `proves`.

## Verification

Positive coverage: store, resolve, and fetch round-trips; recovery after simulated crash replays atomically recovered batches only.

Negative coverage: missing digest fails lookup; interrupted batch leaves no mappings; ObjectId exhaustion fails closed.

Boundary coverage: no runtime claim beyond recorded measurements; no default-path change; ChaosControl campaign evidence stays external and recorded by reference.

## Approved rejected-spike exception

The user selected **Close rejected spike with exemption** on 2026-10-29. The measured decision keeps Redb and changes no default. The exception discharges only the impossible F-series campaign-reference linkage; no backend campaign was executed or inferred.

- Reason: F01–F14 are source-level logic findings, not the proposed storage-fault campaign, and no backend campaign reference exists.
- Recurrence decision: apply once to closing this rejected spike, not to another backend or a later promotion.
- Revisit trigger: a different backend revision, a proposal to change the default or admit production use, or new backend-specific campaign evidence.
- Maintenance owner: Molten storage maintainers.
- Capability non-claim: the retained comparison and optional seam do not prove durability, runtime superiority, deployment readiness, or a ChaosControl campaign.

See [the approved exemption record](evidence/2026-10-29-rejected-spike-exemption.md).
