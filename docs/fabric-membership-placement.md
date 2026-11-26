# Fabric membership and placement runtime

Aspen exposes membership, failure observation, placement, and role assignment as separate fabric facts. The runtime is generic across system extensions, but it does not invent a global cluster truth or service semantics. Existing `molten` module and wire names retain the internal codename.

## Authority boundaries

- A membership view is an immutable, source-scoped snapshot. Static, policy-managed, consistency-backed, and deterministic-simulation profiles retain distinct authority strengths and freshness limits.
- Transport connectivity does not add a member. A node descriptor and eligibility record do not grant a capability. A failure observation does not remove a member. A placement plan does not start a role.
- Failure detectors emit fresh, profile-bound observations. `suspected` and `unavailable` remain bounded observations; they do not prove process death, revoke authority, or transfer ownership.
- Assignments require a separate authority ref, resource reservation, placement-plan ref, service generation, assignment epoch, and fencing token before extension role effects run.

The typed source profile is [`fabric-membership-placement/profile.ncl`](fabric-membership-placement/profile.ncl). Its Nix check exports the positive profile and requires every negative fixture to fail contract evaluation.

## Functional core

`crates/molten-core/src/fabric_membership/` owns deterministic logic only:

- source-profile, view, descriptor, label-authority, compatibility, ordering, and freshness validation;
- failure-observation validation and deterministic reduction that is independent of input order: raw ticks are compared only among detectors sharing one `time_basis_ref` (latest wins, then class precedence, then detector profile ref), and across distinct bases the most severe class wins without comparing ticks;
- capacity accounting, required features and labels, scored preferences, anti-affinity, split-view denial, a pre-check that reports too few distinct anti-affinity values as unsatisfied, and bounded deterministic placement search over candidate sets rather than orderings. Healthy candidates rank strictly ahead of `suspected`/`unavailable` ones regardless of preference score, and `allow_degraded` is only a fallback: search admits zero degraded replicas first and raises that bound one replica at a time, so a plan uses the fewest degraded nodes. A pass whose degraded bound cannot supply enough distinct anti-affinity values (healthy values plus at most that many degraded-only values) is skipped without search, and a non-final pass that exhausts its step bound falls through with residual capacity restored, so the degraded fallback is always reached; only the final pass reports the search limit;
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

`fabric_membership::persistence::Redb` supplies the node-local backend through a capability-derived `DurableStoreRoot`. It stores canonical packed Preserves assignments, pending intents and a same-transaction current role-slot index. `initialize` admits only pure Proposed state. `current` refuses unresolved intents and superseded IDs; intent preparation compares the complete expected assignment, and commit requires its exact pending intent. Reopening never replays a role effect. Successor installation requires an exact Released predecessor ref/epoch, increasing epoch/token and nonregressing service generation. This backend does not grant operator authority or claim cross-host exclusion. Quorum authority and externally enforced fencing remain separate stronger profiles.

A pending intent left by a failed or crashed execution is recovered only explicitly, and never from a caller's claim. `inspect_pending` reads it without replaying its effect. `abort_pre_effect_intent` clears the exact intent and keeps the last committed assignment only when non-execution is definite: the persisted transition has no role effect (`reserve`, `assign`, `acknowledge`), or the owner asks the role lifecycle port (`ExtensionRoleLifecyclePort::observe_intent`) about that exact intent and the port returns `NotExecuted` evidence naming the same assignment, command transition ref, command kind and intent ref. The default port observation is `Unknown`, so after a crash or reopen without intent-bound port evidence the abort fails as uncertain. The addressable-actor placement lifecycle records every activation it refuses before any native effect (an `actor_preview` or `signing.prepare` failure) and reports exactly those as `NotExecuted`; its placement step aborts such an intent immediately, so the next step reads a committed current assignment, while a lost `actor_rewake` outcome stays pending and uncertain.

`reconcile_pending_intent` resolves a possibly executed effect: `ConfirmObservedEffect` commits the exact pending transition only when the lifecycle port returns `Executed` evidence bound to that intent, using the observed effect ref; `QuarantineUncertain` quarantines the last committed assignment, with `uncertain_old_owner` set and a transition ref derived from the intent, only after a `QuarantineAuthorityPort` returns an authorization that echoes the exact request (assignment, committed assignment ref, and intent). No caller-supplied evidence ref is accepted. Entering `Quarantined` persists an owner-computed reconciliation record (`quarantine_reconciliation`) whose basis is the committed quarantine effect, the authorized reconciled intent, or a migrated legacy record. A store written before reconciliation records existed opens unchanged; `reconcile_legacy_quarantine` migrates a `Quarantined` record that has no pending intent and no record, but only with an authorization bound to that exact quarantined assignment and its quarantine transition ref, and never overwrites an existing record.

Quarantine does not brick the role slot: `initialize_quarantine_successor` admits a successor only when the `QuarantineClearance` names the exact quarantined assignment ref and the reconcile ref of its persisted record, and a fence observed through `FencingAuthorityPort` already enforces the successor's advanced epoch and token. The caller does not choose the fence strength: `minimum_quarantine_succession_enforcement` requires a node-local durable fence for a successor on the quarantined owner's node and a quorum-ordered or externally enforced fence for a successor on another node. A fabricated reconciliation ref, an unobservable fence, a fence still at the quarantined owner's epoch and token, or a fence weaker than the derived minimum denies the successor before the role slot changes owner.

## Drain and replacement

Planned drain first stops new work, then performs an extension-owned handoff or checkpoint when configured, stops the role, and releases the reservation after acknowledgement. Exceeding the grace boundary yields an uncertain forced-release decision.

Failure replacement is a different path. It records uncertain old-owner state and requires a successor assignment with an advanced epoch and token. A delayed acknowledgement from the old assignment cannot reactivate a released or replaced role.

## Evidence and readback

Canonical values bind source profile, view and descriptor refs, label authority, failure observations, placement policy and reasons, residual capacity, assignment lifecycle, fencing context, persistence intent, role-effect receipt, and terminal persistence receipt. Operator readback is aggregate and bounded: it exposes current view/member/assignment refs and lifecycle counts, not backend handles, secrets, capability material, or payloads.

## Non-claims

Membership and placement evidence does not prove consensus, global membership truth, process death, capability authority, committed placement, service correctness, or production safety. Simulation determinism is not live authority, and weaker fencing profiles cannot claim stronger distributed exclusion.
