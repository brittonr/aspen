# Fabric membership and placement runtime

Molten exposes membership, failure observation, placement, and role assignment as separate fabric facts. The runtime is generic across system extensions, but it does not invent a global cluster truth or service semantics.

## Authority boundaries

- A membership view is an immutable, source-scoped snapshot. Static, policy-managed, consistency-backed, and deterministic-simulation profiles retain distinct authority strengths and freshness limits.
- Transport connectivity does not add a member. A node descriptor and eligibility record do not grant a capability. A failure observation does not remove a member. A placement plan does not start a role.
- Failure detectors emit fresh, profile-bound observations. `suspected` and `unavailable` remain bounded observations; they do not prove process death, revoke authority, or transfer ownership.
- Assignments require a separate authority ref, resource reservation, placement-plan ref, service generation, assignment epoch, and fencing token before extension role effects run.

The typed source profile is [`fabric-membership-placement/profile.ncl`](fabric-membership-placement/profile.ncl). Its Nix check exports the positive profile and requires every negative fixture to fail contract evaluation.

## Functional core

`crates/molten-core/src/fabric_membership/` owns deterministic logic only:

- source-profile, view, descriptor, label-authority, compatibility, ordering, and freshness validation;
- failure-observation validation and deterministic reduction;
- capacity accounting, required features and labels, scored preferences, anti-affinity, split-view denial, and bounded deterministic placement search;
- explicit propose, reserve, assign, acknowledge, activate, drain, replace, release, fail, and quarantine transitions;
- generation, epoch, token, authority-profile, and enforcement-strength fencing checks;
- bounded drain decisions and live/simulation parity comparison.

The planner receives the complete view, descriptors, current reservations, observations, policy ref, and tie-break order. It returns either an advisory plan with residual capacity and reasons or structured unsatisfied constraints. It performs no filesystem, network, process, clock, policy, persistence, or system-extension effects.

## Imperative shell

`src/fabric_membership/` owns effectful integration:

- static and policy-managed live providers and a deterministic snapshot-stream provider implement one provider contract;
- provider snapshots are independently revalidated and projected to canonical Preserves values;
- the extension role lifecycle port activates, drains, replaces, releases, fails, or quarantines a role only after the pure transition is admitted;
- assignment persistence records intent before an external role effect and commits afterward;
- an intent, role-effect, or commit failure returns an explicit phase and whether an effect may have happened. It is never rewritten as a clean denial or successful assignment.

A durable store, quorum authority, or external fencing service can implement these ports, but the generic shell does not claim their strength. Process-local, node-local durable, quorum-ordered, and externally enforced fencing profiles remain distinct.

## Drain and replacement

Planned drain first stops new work, then performs an extension-owned handoff or checkpoint when configured, stops the role, and releases the reservation after acknowledgement. Exceeding the grace boundary yields an uncertain forced-release decision.

Failure replacement is a different path. It records uncertain old-owner state and requires a successor assignment with an advanced epoch and token. A delayed acknowledgement from the old assignment cannot reactivate a released or replaced role.

## Evidence and readback

Canonical values bind source profile, view and descriptor refs, label authority, failure observations, placement policy and reasons, residual capacity, assignment lifecycle, fencing context, persistence intent, role-effect receipt, and terminal persistence receipt. Operator readback is aggregate and bounded: it exposes current view/member/assignment refs and lifecycle counts, not backend handles, secrets, capability material, or payloads.

## Non-claims

Membership and placement evidence does not prove consensus, global membership truth, process death, capability authority, committed placement, service correctness, or production safety. Simulation determinism is not live authority, and weaker fencing profiles cannot claim stronger distributed exclusion.
