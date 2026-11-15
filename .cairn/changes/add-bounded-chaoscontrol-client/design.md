## Context

Reuse live-process election, committed request, quorum read, snapshot catch-up, partition, process crash and reopen. The first native profile is a fixed three-participant scenario; phases are coarse commands and unsupported fine-grained schedules fail closed. All retained identities use canonical packed Preserves and BLAKE3.

## Decisions

1. Input validation admits only the bounded static profile and strict phase prefixes. Each phase has a stable request identity and consistency ordering key. Reduction drops suffixes only; it never removes a prerequisite. Full qualification requires the final fault-off horizon.
2. The shell calls real child execution. OutcomeClass is transport-level evidence only; independent per-participant observations decide semantic results. No-op, missing participant, unknown fault application, missing local recovery provenance, quarantine or short horizons cannot pass.
3. Each recovery participant measures its own event-loop iterations after restart and term advancement, not aggregate throughput or wall time. Native loop ticks are not guest instruction ticks. Local reopen is recorded before peer ingress; peer repair and absence require explicit owner observations and are not inferred from eventual reads.
4. Storage observations reuse storagefault classifications and sticky quarantine; lack of sync evidence remains unavailable, never a machine-loss acknowledgement.
5. The intentional duplicate-application control invokes the real application handler a second time after an admitted commit. Deployment precedes execution; reached evidence is emitted only after a readback detects the duplicate. The independent apply-once property owns detection. Suppression invalidates qualification.
6. Guest consumption must use ChaosControl MultiNodeGuestIntake and its existing process bundle closure mechanism, single-vCPU per guest only. The current native test executable depends on test-only setup, Tokio processes, Redb, Iroh and host filesystem/network services; it is not a qualified guest binary. Missing binary/image/kernel/closure identities block guest handoff.

## Non-Claims

This profile observes one control-plane request and bounded recovery only. It does not prove arbitrary histories, physical power loss, linearizability, exactly-once external effects, KVM replay, SMP support, release eligibility or production readiness. Original-build reruns require identical immutable source/binary bindings; changed candidates require newly measured bindings.
