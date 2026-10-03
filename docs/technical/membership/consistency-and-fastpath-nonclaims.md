# Consistency and Fastpath Non-Claims

Membership, consistency, and fast-path modeling share vocabulary such as views, epochs, and acknowledgements. They do not share an automatic authority upgrade. This article relates the [membership runtime](../../fabric-membership-placement.md) to the [consensus fast-path hazard model](../../consensus-fastpath-hazard-model.md). It assumes those governing documents and is part of the [Technical companion](../README.md), not a proposal to select a live acceleration engine.

## A source classification is not a consensus implementation

Membership profiles include `ConsistencyBacked` and authority strengths such as `ConsistencyOrdered`. Those types preserve the provenance and declared strength of a supplied view. They do not make the membership core a consensus service. The core accepts explicit inputs and returns deterministic admission, reduction, placement, and lifecycle decisions without transport or persistence effects.

The inspected [`PolicyManagedMembershipProvider`](../../../src/fabric_membership/adapters.rs) accepts policy-managed or consistency-backed snapshots. Replacement requires the same provider kind and profile reference and an advancing view epoch. The adapter stores and returns the supplied snapshot; these operations do not demonstrate how an upstream consistency authority obtained agreement. Calling this adapter “consistency-backed” cannot supply missing quorum or durability evidence.

Likewise, a planner's refusal to combine conflicting membership views is not a consensus algorithm. It preserves uncertainty at an authority boundary. An external consistency service might resolve a membership decision, but neither the decision's shape nor its canonical reference establishes that the resolution happened.

## What the fast-path artifact actually contains

The [checked configuration](../../../config/consensus-fastpath/profile.ncl) defines three- and five-replica crash-fault model profiles. The configured bounds are eight commands, four keys, four views, and 64 steps. These are finite exploration parameters, not service capacity or throughput limits. The contract admits only pure-model or deterministic-simulation selection and a pure-model-only claim profile.

[Rust profile admission](../../../src/fabric_consistency/fastpath/profile.rs) separately rejects live and production selection and unsupported fault models, counts, bounds, or claim profiles. It checks the reference cohort and ordering prerequisites. The external paper and pinned artifact identify the design reference; their results do not transfer into proof of Molten's implementation.

The quorum calculation is concrete: majority is `floor(n / 2) + 1`, and superquorum is `floor(3n / 4) + 1`. The admitted counts therefore yield majority/superquorum pairs of 2/3 and 3/4. For three replicas, one failed replica can leave majority progress possible while preventing the fast path. That is a structural model observation, not a measured availability result.

## Stable-view evidence is conjunctive

[`evaluate_stable_view`](../../../src/fabric_consistency/fastpath/stable.rs) checks equality between the operation identity used by the two paths, conflict-free classification, same-attempt view coordinates for acknowledgements, enough distinct replica identifiers, and compatible promises from exactly the active proposer set. An acknowledgement for another operation or either different view coordinate prevents the same-view acknowledgement set from forming.

A membership view does not supply these promises. A node being eligible to host a role does not establish that it acknowledged this command in this consensus attempt. The identity record includes command and session context alongside group, generation, schema, policy, authority, resource, and engine-epoch context; matching only the node list is insufficient.

There is a scoped distinction between intended composition and a helper's checks. The [governing stable-view boundary](../../consensus-fastpath-hazard-model.md#stable-view-boundary) calls for matching acceleration and base views. The stable helper compares each acknowledgement and promise to the attempt's respective acceleration and base fields, but does not directly require the attempt's two fields to equal each other. The [recovery helper](../../../src/fabric_consistency/fastpath/recovery.rs) does require equality when resuming normal operation. This article therefore does not claim that an arbitrary standalone call to the stable helper independently establishes the entire matching-view precondition.

## Illustrative view-straddled attempt

Consider the three-replica profile and an attempt at acceleration/base view 7. Two replicas acknowledge in view 7; a third acknowledgement arrives carrying view 6. Counting three received messages would appear sufficient numerically, but the stable helper rejects the mixed-view set before comparing its size with the superquorum. Two valid acknowledgements alone do not reach the required three.

A fresh membership snapshot containing all three nodes cannot repair that result. Nor can a detector's `Recovered` observation rewrite the old acknowledgement's view. If the original path is available, the model exposes fallback; if not, its fallback reason records original-path unavailability. None of those outcomes is evidence that a live transport sent, persisted, or executed anything.

Recovery introduces another sequencing boundary: it pauses for base-view change, preserves accepted commands in an agreed recovery set, requires a marker, and admits a matching later normal view. The pure recovery transition retains set and phase facts; the governing requirement to commit recovered commands through the original path is a composition obligation, not an external effect performed by that helper.

## Verification and review guidance

Suggested review separates four evidence questions: whether the membership source is admitted, whether assignment authority permits effects, whether a bounded model schedule satisfies its invariants, and whether a live integration actually enforces its claimed guarantees. Keep profile selection, finite bounds, source revision, and unexplored alternatives attached to model readback.

A focused model review can examine mixed-view acknowledgements, missing proposer promises, insufficient superquorums, and interrupted recovery without making performance claims. The in-memory `ApplicationLedger` suppresses repeated command application and session replies in its retained sets; this is not an exactly-once claim across crashes or external effects. No model execution or live consistency experiment was performed for this documentation task.

## Limits and non-claims

Canonical model evidence does not prove transport, durability, production linearizability, Byzantine tolerance, throughput, latency, or release readiness. Cryptographic verification adds no missing membership or runtime authority, as the [identity boundary](../../fabric-cryptographic-identity.md) states. A consistency-backed source, an admitted model, and a successful local transition remain different facts requiring different evidence.

## Sources

- [Fabric membership and placement runtime](../../fabric-membership-placement.md)
- [Consensus fast-path hazard model](../../consensus-fastpath-hazard-model.md)
- [Fabric cryptographic identity adapters](../../fabric-cryptographic-identity.md)
- [Membership provider adapter](../../../src/fabric_membership/adapters.rs)
- [Fast-path configured bounds and claims](../../../config/consensus-fastpath/profile.ncl)
- [Fast-path configuration contract](../../../config/consensus-fastpath/contracts.ncl)
- [Rust profile admission and quorums](../../../src/fabric_consistency/fastpath/profile.rs)
- [Stable-view and convergence model](../../../src/fabric_consistency/fastpath/stable.rs)
- [Recovery transition model](../../../src/fabric_consistency/fastpath/recovery.rs)
