# Suspicion and Source-Scoped Views

A failure detector produces a bounded observation, not a verdict about existence or authority. This article develops that distinction through Molten's admission and reduction rules. Prerequisites are the [membership runtime](../../fabric-membership-placement.md) and its separation of membership, failure observations, placement, and assignment. Return to the [Technical companion](../README.md) for the surrounding articles.

## A view is an input with provenance

`MembershipSourceProfile` distinguishes static, policy-managed, consistency-backed, and deterministic-simulation providers. Its `authority_scope`, `authority_strength`, and `max_view_age_ticks` delimit what a supplied snapshot represents. `MembershipView` adds source evidence, authority and eligibility-policy references, an epoch, observation time, expiration time, and the member set. The [core validator](../../../crates/molten-core/src/fabric_membership/mod.rs) checks these values without reading a clock or invoking an authority service.

Freshness has two independent bounds. A view is stale when caller-supplied `now_ticks` exceeds `valid_until_ticks`, or when its age exceeds the source profile's maximum. Future observation times and reversed validity intervals are separately rejected. Equality with either freshness limit is not rejected by those strict greater-than checks. Tick values are explicit inputs; this code does not infer a common real-world time base from their numeric equality.

Source scope matters when two providers disagree. Equal epochs from different sources are not evidence of one global order. The planner accepts explicit conflicting-view references and returns an unsatisfied placement outcome rather than promoting unreconciled views into a common authoritative view. That is a denial of an inference, not an automatic reconciliation algorithm.

## Validating observations before reduction

A `FailureDetectorProfile` binds a profile reference, a time-basis reference, a maximum observation age, and mandatory non-claims. A `FailureObservation` names its subject, detector profile, class, observation and expiration ticks, confidence in basis points, and supporting-event references.

The reducer validates that subjects belong to the admitted view and that each observation names a supplied detector profile. It rejects future or stale observations, confidence above 10,000 basis points, absent supporting events, and malformed event references. Detector profiles have positive age windows and distinct references. Validation errors cause the function to return issues rather than a successful partial reduction. This is important: an invalid alarming observation is not silently discarded while the remaining inputs produce an apparently clean answer.

The required observation non-claims exclude process-death proof, membership mutation, authority revocation, and ownership transfer. Confidence does not override these exclusions. A 10,000-basis-point observation remains an observation from its declared detector context, not a capability to terminate another owner's authority.

## What deterministic reduction actually selects

For each subject, `reduce_failure_observations` prefers a later `observed_at_ticks`. At equal timestamps it prefers the greater class precedence: `Unknown`, `Reachable`, `Recovered`, `Suspected`, then `Unavailable`. Confidence is validated but is not used as a ranking weight. The reduced value retains the subject, chosen class, observation time, and detector-profile reference.

There is an important qualification to “deterministic.” With the same ordered inputs, the function returns the same reduction. If two observations have identical timestamps and classes but different detector-profile references, the current entry is retained: the first encountered profile reference survives. The implementation does not sort equal candidates by detector identity. Thus this article does not claim permutation-invariant provenance for all input multisets. The [existing test](../../../crates/molten-core/src/fabric_membership/tests.rs) establishes the equal-time `Reachable` versus `Suspected` case and unchanged membership, not every possible tie.

## Illustrative partition and recovery

Assume an admitted view contains `node-a`, `node-b`, and `node-c`. At current tick 120, two otherwise valid observations about `node-b` both have observation tick 110 and expiration tick 140. One says `Reachable`; the other says `Suspected`. The reduced class is `Suspected`, even if the reachable observation has higher confidence. The member set remains unchanged.

A later valid `Recovered` observation at tick 115 supersedes that suspicion because recency is compared before class precedence. This is not a mathematical proof that `node-b` is healthy; it is the deterministic answer to which admitted observation governs this reduction. If the current tick moves beyond an observation's validity bound, admission fails rather than treating old recovery as indefinitely fresh.

Placement then interprets the reduced class through its own requirements. In the inspected candidate filter, `avoid_suspected` excludes suspected or unavailable candidates when `allow_degraded` is false. A degraded policy can permit consideration of those candidates. Neither branch changes membership or transfers an existing role. That later action requires assignment authority and fencing, as the governing runtime document states.

## Verification and review guidance

Suggested review cases include future timestamps, exact expiration boundaries, an unknown subject, equal-time class conflicts, later recovery, and equal-time/equal-class observations from different profiles. Keep the membership snapshot unchanged while checking the reduced output. Reviewers should also preserve detector time-basis provenance instead of assuming ticks from unrelated detectors are globally synchronized.

The [provider adapters](../../../src/fabric_membership/adapters.rs) offer deterministic snapshot streams and advancing live snapshots, but those mechanisms do not measure network timing. Source inspection and test reading were performed for this article; no detector experiment or test execution is claimed.

## Limits and non-claims

Deterministic reduction is not a failure-detector completeness or accuracy theorem. The equal-candidate provenance caveat narrows the [governing document's broad deterministic wording](../../fabric-membership-placement.md#functional-core) to the behavior observed in the [reducer](../../../crates/molten-core/src/fabric_membership/mod.rs); it is not resolved here by an invented tie rule. Cryptographic verification likewise does not supply the missing membership or authority decision, as the [identity boundary](../../fabric-cryptographic-identity.md) explains.

## Sources

- [Fabric membership and placement runtime](../../fabric-membership-placement.md)
- [Fabric cryptographic identity adapters](../../fabric-cryptographic-identity.md)
- [View, detector, reduction, and placement implementation](../../../crates/molten-core/src/fabric_membership/mod.rs)
- [Failure-observation regression cases](../../../crates/molten-core/src/fabric_membership/tests.rs)
- [Snapshot provider implementations](../../../src/fabric_membership/adapters.rs)
