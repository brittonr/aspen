# Preserve replica fault domains during cleanup

## Why

F05 demonstrates an unsafe cleanup plan at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

The executed counterexample is `audit_cleanup_preserves_required_fault_domains`. Desired replicas, minimum verified replicas, and minimum fault domains each equal two. Generation is 1, membership epoch is 2, and placement epoch is 3.

Three current verified replicas hold the same protected content. `peer-a` and `peer-b` occupy `zone-a`. `peer-c` occupies `zone-b`. Every replica is unpinned and has cleanup clearance. The content rule includes cleanup authority, and cleanup after handoff is enabled.

Expected: select A or B, so both domains remain. Actual: reverse peer-ID order selects C, so the proposed remainder has only `zone-a`.

The counterexample executed the pure planner, not deletion. Static shell review shows retention authorization before content cleanup. Current conformance adapters can reject the unsafe action. This finding proves neither executed deletion nor production data loss.

## Proposed scope

Evaluate replica count and fault-domain policy against the cumulative remainder before each cleanup selection. Preserve separate cleanup authority, retention clearance, pin checks, and epoch fencing.

The current consumer is the content-replication cleanup service. The Molten content-replication maintainers own the change. The durable capability is deterministic policy-preserving cleanup with normal repository tests.

## Evidence and limits

The audit reports 359 passing core baseline tests. Its separate harness reports 41 passing controls and eight failing regression assertions. F05 is an executed unsafe-plan counterexample. Six other findings are static only.

This package includes the trigger and does not require ignored scratch evidence. No planning commands or acceptance gates ran. This proposal grants no implementation permission or release approval. A safe plan does not grant deletion authority or prove permanent availability.
