# Bound terminal scheduler retention

## Why

F11 is an executed defect at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
Set `max_runnables` to one. Wake A, select A, and complete A.
Then Wake B, select B, and complete B in the same generation.
The state retains two records despite the active limit of one. Repetition continues to grow the vector.
Expected behavior is finite retained storage with explicit replay-safe reclamation.

`crates/molten-core/src/fabric_time/scheduler/mod.rs` counts only nonterminal records for Wake admission.
Complete, Cancel, and generation cleanup keep terminal records in the same vector.
The current consumers are the scheduler service and its preallocated capacity shell.

## Proposed change

Introduce a named finite retention contract consistent with the admitted capacity plan.
Reclaim terminal records deterministically without reclaiming active work.
Fence fresh occurrences independently of tombstone presence so old callbacks cannot resurrect evicted work.
Use explicit fresh occurrence IDs or generation fences, with checked exhaustion behavior.

## Ownership and durable capability

Molten fabric-time maintainers own retention policy, occurrence identity, adapters, and regression tests.
The immediate outcome is bounded terminal storage without ABA reuse.
The durable capability is a repeatable bounded-lifetime contract across state, replay, and receipts.
Existing scheduler consumers adopt the contract through explicit compatibility admission, not a new dependency mandate.

## Evidence and non-claims

The audit executed `audit_terminal_runnables_remain_bounded` and observed two retained records under active limit one.
Its core baseline reports 359 passing tests. Its harness reports 41 passing controls and eight failing regression assertions.
Eight audit findings have executed counterexamples. Six other findings have static evidence only.
ABA protection is a required design constraint, not an executed audit finding.
The sequence and result here do not depend on ignored scratch files for meaning.
This planning pass ran no tests or commands and grants no implementation permission.

Finite state does not establish measured memory savings, zero allocation, global liveness, fairness, or release readiness.
