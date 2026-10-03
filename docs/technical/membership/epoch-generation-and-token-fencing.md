# Epoch, Generation, and Token Fencing

Molten keeps several kinds of freshness and authority context explicit rather than compressing them into one “version.” This article explains assignment generation, epoch, token, profile, and state checks. It assumes the [membership lifecycle contract](../../fabric-membership-placement.md); the [Technical companion](../README.md) provides adjacent discussions of planning and observation.

## Similar numbers are not interchangeable

A membership view epoch identifies progression within its source context. An assignment separately carries `service_generation`, `assignment_epoch`, and `fencing_token`. Cryptographic identity has its own generation-scoped handles and rotation rules. The [identity documentation](../../fabric-cryptographic-identity.md) does not equate those key generations with assignment epochs or service generations.

The inspected assignment implementation makes these distinctions operational. `apply_assignment_command` checks assignment identity and requires the command's generation, epoch, and token to equal the current assignment's values. It also validates the transition reference, rejects a reference already present in `transition_refs`, and enforces the allowed state transition. A numerically larger command value does not automatically authorize a transition: inequality produces the corresponding stale-context issue.

This is why “newer” cannot be used as a universal admission rule. A successor assignment is a different proposal with its own identity and predecessor context. An operation against an existing assignment is expected to match the already admitted scope, not unilaterally advance it.

## Two checks with different jobs

[`validate_assignment_authority`](../../../crates/molten-core/src/fabric_membership/transition.rs) compares the assignment with an `AssignmentAuthoritySnapshot`. It validates the fencing profile, requires profile and authority references to match, compares assignment epoch and token with the enforced values, and rejects an enforcement level weaker than the required level. The [assignment shell](../../../src/fabric_membership/shell.rs) performs this check before intent persistence or lifecycle effects.

`validate_fenced_operation` addresses use of an assignment rather than a lifecycle command. It requires an active assignment; matching assignment identity and service generation; agreement among assignment, operation, and profile references; matching authority references; and agreement among operation, assignment, and enforced epoch/token values. It also checks that the profile's enforcement meets the operation's requirement.

`FencingEnforcement` orders `ProcessLocal`, `NodeLocalDurable`, `QuorumOrdered`, and `ExternallyEnforced`. The comparison expresses the declared strength required for admission. It does not construct a quorum, update an external storage fence, or prove that the selected port actually enforces its declaration. The profile includes an effect-port reference, but the pure validator performs no external call. The governing document therefore preserves those enforcement classes instead of presenting them as equivalent distributed exclusion.

## Illustrative stale owner

Suppose assignment A is active with service generation 3, epoch 8, and token 41. A replacement authority has advanced its enforced context to epoch 9 and token 42. A delayed operation from A still carries epoch 8 and token 41.

Even if A's local object still says `Active`, supplying the advanced enforced values to `validate_fenced_operation` rejects the operation for both epoch and token mismatch. Its local state is not enough. Conversely, supplying stale enforced values could not establish that the external authority has actually advanced; the pure function only reasons over supplied facts. The integration's obligation to obtain and enforce real current authority cannot be replaced by the success of an in-memory equality check.

A second independent defense concerns lifecycle resurrection. If A has reached `Released`, a delayed `Acknowledge` command is invalid even when its numeric context matches A. Released and quarantined states are terminal according to the inspected state helper. Numeric freshness, state legality, and authority strength protect different boundaries.

## Replacement contract and a scoped implementation gap

The [governing drain-and-replacement section](../../fabric-membership-placement.md#drain-and-replacement) describes failure replacement as requiring a successor assignment with an advanced epoch and token. The inspected [`AssignmentProposal` and `propose_assignment`](../../../crates/molten-core/src/fabric_membership/transition.rs) support a predecessor assignment reference and predecessor epoch, require the predecessor fields together, and reject an assignment epoch that does not advance beyond that predecessor epoch.

However, the proposal has no predecessor-token field. This function checks that the supplied fencing token is nonzero, but cannot compare it with a predecessor token. The intended advanced-token contract is therefore broader than this constructor's local check. Authority and operation validators compare against supplied enforced tokens, but those comparisons are not evidence that the constructor independently guarantees monotonic successor tokens. This documentation does not resolve that gap, add a new protocol, or modify implementation.

Drain has a related but distinct boundary. `evaluate_drain` returns ready-to-release only after new work is stopped, required handoff is satisfied, the role is stopped, and release is acknowledged. If progress is incomplete at or beyond the deadline, it returns `ForceReleaseUncertain`. Capacity release separately requires a released assignment and matching reservation reference, node, and epoch. An elapsed deadline is not proof of old-owner death.

## Verification and review guidance

The existing core tests cover delayed acknowledgement after release, weak fencing offered for a quorum-ordered requirement, and operations checked against advanced enforced epoch/token values. Suggested review should independently vary generation, epoch, token, profile, authority, state, and strength rather than testing only a single generic “stale” input.

For replacement, inspect the actual authority provider's token progression and enforcement mechanism; constructor success alone is insufficient evidence. No live fence, external store, or test execution is reported by this article. The source-level limitation above remains explicit for integration review.

## Limits and non-claims

Fencing metadata is not an external fence. Generation equality is not signature verification. Duplicate transition-reference rejection is not exactly-once role execution. Neither process-local validation nor declared quorum strength proves distributed exclusion or production safety without corresponding operational enforcement.

## Sources

- [Fabric membership and placement runtime](../../fabric-membership-placement.md)
- [Fabric cryptographic identity adapters](../../fabric-cryptographic-identity.md)
- [Assignment, fencing, and drain implementation](../../../crates/molten-core/src/fabric_membership/transition.rs)
- [Assignment authority shell](../../../src/fabric_membership/shell.rs)
- [Fencing and lifecycle regression cases](../../../crates/molten-core/src/fabric_membership/tests.rs)
- [Declared membership and fencing profile](../../fabric-membership-placement/profile-template.ncl)
