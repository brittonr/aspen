# Placement Versus Committed Assignment

Placement asks where roles could fit under explicit constraints. Assignment asks whether a particular role transition is authorized and what happened when its effects were attempted. This article assumes the [membership runtime](../../fabric-membership-placement.md) and explains the implementation boundary between those questions. It belongs to the [Technical companion](../README.md), not to a new scheduling policy.

## The planner's result is deliberately advisory

The pure [`plan_placement`](../../../crates/molten-core/src/fabric_membership/mod.rs) function receives an admitted view and a request containing requirements, current assignments, reservations, observations, time, conflicting-view references, and tie-break order. It validates the request, reduces failure observations, and computes residual capacity. None of these steps reserves operating-system resources or starts an extension.

Residual capacity is componentwise CPU, memory, and storage accounting. Unreleased reservations are subtracted with checked arithmetic; excess reservations become validation issues rather than negative available capacity. Candidate selection requires sufficient residual resources, required runtime features, and hard labels whose authority meets each constraint. Preferred labels contribute scores only when their values and authority thresholds match. Scored candidates are ordered by descending score, explicit tie rank, then node identifier.

Search also considers active assignments for the same service and role kind when establishing occupied nodes and anti-affinity state. The governing document describes bounded deterministic search rather than an unrestricted optimization proof. A successful plan records selected roles, reasons, residual capacity, whether it is degraded, and `advisory_only: true`. An unreconciled conflicting view produces an unsatisfied result, not an arbitrary choice of the more convenient membership set.

An advisory result is useful precisely because it can be inspected before effects. It answers a counterfactual under supplied data. It does not atomically exclude a second planner from choosing the same residual capacity, refresh its own membership view through a network request, or certify that external capacity still matches a descriptor.

## Assignment adds identity, authority, and lifecycle

An `AssignmentProposal` separately binds extension, service, role, node, service generation, assignment epoch, fencing token, fencing profile, reservation, plan, and authority references. [`propose_assignment`](../../../crates/molten-core/src/fabric_membership/transition.rs) validates proposal shape and creates a `Proposed` assignment; it does not turn a plan directly into running work.

The normal transition chain is `Proposed → Reserved → Assigned → Acknowledged → Active`. Acknowledgement and activation are distinct so that possession of assignment information is not mistaken for completed role activation. Other commands model drain, replacement, release, failure, and quarantine. A delayed acknowledgement cannot move a released assignment back to life because that state/command pair is absent from the transition table.

The shell's [`execute_assignment_command`](../../../src/fabric_membership/shell.rs) first validates the current assignment against an authority snapshot, then applies the pure command. A clean denial at this stage precedes persistence and role effects. If admitted, it records an intent, invokes the relevant lifecycle effect, commits persistence, and constructs canonical transition evidence. Reserve, assign, and acknowledge have no lifecycle-port call in the inspected dispatcher; activation, drain, replacement, release, failure, and quarantine do.

## Illustrative failure after activation

Consider a plan selecting `node-b` for a replica. A separately prepared assignment reaches `Acknowledged`, and an activation command passes authority and transition validation. Intent persistence returns a valid reference. The lifecycle port activates the role and returns valid effect evidence. Persistence then fails while committing the transition.

The result is not a rejected plan and not a successfully committed assignment. The shell returns `Uncertain` with phase `CommitPersistence`, the intent and effect references, and `effect_may_have_happened: true`. This preserves the critical possibility that the role is already running even though the assignment record did not reach its intended terminal persistence outcome.

Blindly retrying the original activation is not justified by the deterministic planner or the presence of a transition reference. Duplicate transition-reference rejection applies to the assignment state supplied to the pure transition; it is not a distributed external-effect deduplication protocol. The concrete persistence and lifecycle integration must resolve uncertainty within its own real authority and recovery contract. This article does not invent such a protocol.

The same care applies to malformed evidence after an effect. The shell treats a malformed role-effect reference as uncertainty, not as proof that the effect never occurred. Even canonicalization failure after persistence commit remains uncertain. These branches retain the causal order instead of rewriting operational failure into clean admission denial.

## What a commit proves here

`Committed` is an outcome of the shell protocol with its chosen ports. The included [`InMemoryAssignmentPersistence`](../../../src/fabric_membership/adapters.rs) stores assignments and receipt references in process memory. Its use demonstrates the interface and failure ordering; it does not provide durable restart recovery or quorum ordering. The [governing documentation](../../fabric-membership-placement.md) explicitly leaves durable stores and stronger authorities to implementations of the ports.

A canonical transition binds evidence about the assignment process. It is neither a substitute for external fencing nor proof of extension service correctness. Signing such evidence would not supply those missing claims either; the [identity documentation](../../fabric-cryptographic-identity.md) keeps verification separate from authority admission.

## Verification and review guidance

Review the existing shell tests for successful activation, stale-authority denial before effects, failed role effects, failed commit after successful effects, and malformed effect evidence. Suggested integration review should follow one transition through intent, effect, commit, and readback using the actual backend, recording which uncertainty branch occurs at each failure boundary.

No such backend exercise was executed for this article. The verified documentation basis is source inspection and existing assertions, not production persistence evidence.

## Limits and non-claims

The planner is not a reservation transaction. The assignment shell is not an exactly-once engine. A committed local adapter outcome does not imply durable or distributed commitment, and neither planning nor assignment evidence establishes production readiness.

## Sources

- [Fabric membership and placement runtime](../../fabric-membership-placement.md)
- [Fabric cryptographic identity adapters](../../fabric-cryptographic-identity.md)
- [Placement implementation](../../../crates/molten-core/src/fabric_membership/mod.rs)
- [Assignment state machine](../../../crates/molten-core/src/fabric_membership/transition.rs)
- [Intent/effect/commit shell](../../../src/fabric_membership/shell.rs)
- [In-memory persistence mechanism](../../../src/fabric_membership/adapters.rs)
- [Assignment uncertainty regression cases](../../../src/fabric_membership/parts/tests/p001/body.rs)
