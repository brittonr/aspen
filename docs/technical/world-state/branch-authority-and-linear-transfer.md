# Branch Authority and Linear Transfer

Branching state does not branch a live capability. A captured world may preserve public observation identities, but activation requires current authority and realization of the selected branch mode. This article separates portable policy, product obligations, durable ownership, and activation outcomes. The [world-branch-authority contract](../../world-branch-authority.md) is authoritative; the [Technical companion](../README.md) situates it alongside capture and branch-head mutation.

## Policy class is not an executable capability

Basalt supplies portable policy classes and obligations. Molten owns token verification, current revocation and replay observations, scope normalization, capability derivation, durable ownership, adapter selection, and execution. The pure core maps explicit product facts into policy requests and validates the resulting obligation plan. It performs no ambient file, network, clock, secret, token, or persistence operation.

The closed modes have deliberately different realization requirements. Copyable authority requires an independently current destination grant; attenuation requires a strictly narrower scope. Replacement requires a new destination grant before activation. Simulation-only requires an exact deterministic simulation adapter without live fallback. Promotion-gated authority requires current promotion admission and committed reservations but does not authorize dispatch. Non-branchable authority denies activation. Linear authority is the case where copying would be especially misleading: it requires a generation-fenced transfer and source inactivity before destination activation ([mode definitions](../../world-branch-authority.md#closed-modes)).

These classes should not be flattened into an “allowed” flag. An allowed plan identifies obligations still to be realized, and those obligations differ materially between an attenuated grant and a linear handoff.

## The shell's linear transfer sequence

[execute_world_branch_authority](../../../src/world_branch_authority/service.rs) observes policy and authority, constructs a plan, and publishes a plan receipt. A denied plan stops before realization. For a linear plan, the shell first observes ownership and rejects crossed capability identity, generation zero, an inactive source, an already active destination, or an invalid observation reference.

The operation identity binds the plan, capability, and observed generation under a dedicated BLAKE3 domain. The expected successor generation is computed with checked addition. The shell then calls `transfer` once with that operation identity and expected generation. A committed observation proceeds; a denied transfer fails; an unknown transfer invokes `reconcile_transfer` rather than issuing another transfer.

The returned observation must match the operation and successor generation, with both source and destination inactive. This intermediate state is significant: transfer completion is not destination activation. The shell re-observes durable ownership before proceeding and invalidates the realization if capability, generation, or inactivity no longer agrees.

Only after realization does it re-observe policy and authority and invoke the pure activation decision. The [activation core](../../../crates/molten-core/src/world_branch_authority/activation.rs) checks identity binding, destination scope, current facts, evidence bounds and uniqueness, exclusion of bearer material, and mode-specific obligations. Linear admission requires source inactivity, a transfer generation, and a current destination grant. These are checks over supplied observations, not an independent proof of storage exclusivity.

## Worked failure: transfer succeeded, acknowledgement disappeared

Consider an illustrative capability owned by source branch S at generation 9, with destination D inactive. A plan produces operation X. The transfer adapter durably advances ownership to generation 10 and deactivates S, but its acknowledgement is lost.

The shell receives `Unknown`. Reissuing the transfer could conflate a completed mutation with a fresh request, so it instead reconciles X. If reconciliation returns an observation for X at generation 10 with both branches inactive, the shell can continue to the ownership and current-policy rechecks. An observation for operation Y, generation 11, or an active source is not “close enough”; it fails the transfer boundary.

Now suppose policy is revoked after the transfer but before activation. The second policy/authority observation denies activation even though transfer realization succeeded. This leaves a distinction an operator must preserve: the transfer may be complete while destination activation is denied. Treating the earlier plan receipt as reusable authority would erase precisely this safety boundary.

A second uncertainty can occur after activation admission. The shell calls `activate` once; if its outcome is unknown, it calls `reconcile_activation` once and writes a separate outcome receipt. An unresolved outcome remains unknown rather than triggering a second activation attempt.

## Evidence, confidentiality, and promotion

Plan, activation-admission, and activation-outcome receipts answer different questions. None carries bearer authority. The [confidentiality contract](../../world-branch-authority.md#evidence-and-confidentiality) restricts canonical evidence to metadata identities, excluding tokens, credentials, private keys, raw capability paths, private policy bodies, and secret entropy.

Promotion is an adjacent but separate boundary: committed reservation admission is not effect dispatch. Its typed admission explicitly keeps dispatch unauthorized. A branch activation report therefore cannot be used as evidence that an external effect was released or that its handler succeeded.

## Review and verification guidance

Review adapters for the exact meaning of ownership generation, source inactivity, and reconciliation identity. Suggested scenarios are a crossed operation observation, policy revocation between realization and activation, ownership drift during recheck, and an unknown activation outcome. Assert the number and order of mutation attempts, but also inspect the durable state and separate receipts. These are proposed verification scenarios, not executions performed for this article.

The standalone effect commands fail closed without admitted runtime adapters. Their denial receipts should not be mistaken for evidence that a production transfer path has been exercised.

## Limits and non-claims

Observation-first reconciliation is not an exactly-once guarantee. The generic shell depends on truthful, current adapter observations and does not itself implement every durable ownership mechanism. A linear plan does not prove exclusivity; activation admission does not prove activation success or future enforcement. Simulation admission proves neither live parity nor host confinement. No receipt establishes release eligibility or production readiness.

## Sources

- [World branch authority](../../world-branch-authority.md)
- [World commits and historical authority observations](../../world-commit.md)
- [Authority execution and reconciliation shell](../../../src/world_branch_authority/service.rs)
- [Pure activation admission](../../../crates/molten-core/src/world_branch_authority/activation.rs)
- [Technical companion](../README.md)
