# Diagnosing uncollectable and unproven objects

Mode: Troubleshooting

An object can be retained because a dependency is known, or because the evidence needed to rule out dependencies is missing. Those are different incidents with different owners. Preserve the denial and its scope before investigating; do not remove state or relax an admission to obtain a passing result.

The symptoms below are source-review cases, not reproduced incidents. No runtime verification was performed for this handbook batch. Use the guarded inspection procedure in [inspecting pins before a GC plan](inspecting-pins-before-a-gc-plan.md) to collect a candidate explanation without invoking destructive evaluation.

## Symptom: an apparently obsolete object is still pinned

**Discriminating evidence:** `active-pins-present` occurs when evaluation finds pin references, and also in candidate explanation when matching pins exist. Inspect the pin's object, class, source, reason, and owner. Compare active store records with historical exported artifacts: the fixture intentionally preserves `pin.preserves` after removing its own live pin.

**Safe next action:** ask the pin's workflow owner whether the protected replay, session, rollback, evidence, legal, or operator dependency remains active. Record the answer against the exact reference. A newer head or an additional replica does not cancel unrelated roots.

**Stop condition:** an active dependency or unresolved ownership remains. Do not manually remove the pin, infer automatic expiry from an expiry reference, or repeat destructive attempts hoping the blocker disappears.

## Symptom: no pin appears, but deletion remains unproven

**Discriminating evidence:** `incomplete-reference-proof` means the evaluation's completeness input is false; `retention-candidate-no-known-evidence` means the explanation found no matching records in its known evidence families. Neither says “safe to delete.” Check whether optional explanation filters hid other scopes, whether the correct root was inspected, and whether the object reference came from actual inventory.

For world objects, inspect `missing_classes`, `unresolved_remote`, edge completeness, and attribution completeness. All classes being present is insufficient when a lease is unresolved or graph inventories are incomplete.

**Safe next action:** obtain the missing owner observations, including explicitly empty classes. Keep “not observed” distinct from “observed empty.” A complete world report is ANDed with the preexisting retention completeness flag, so also investigate the wider index.

**Stop condition:** any required observation is missing. Do not set a completeness flag just because a local directory looks empty.

## Symptom: remote evidence exists but the plan denies clearance

**Discriminating evidence:** local evaluation can report `remote-cache-refs-present`; the plan regression covers `remote-clearance-evidence-missing`. Inspect the intended peer, candidate/action scope, remote references, and actual admission/clearance records. A plan reference supplied where a clearance is expected is tested as unreadable clearance evidence, not accepted as an equivalent artifact.

**Safe next action:** route the missing or stale observation to the remote-clearance owner. Retain uncertain, contradictory, or unavailable lease roots. Preserve a received response and its import context before deciding whether additional communication is safe.

**Stop condition:** remote effect or clearance state is uncertain. A timeout is not evidence of rejection or completion; do not unconditionally resend uncertain operations or substitute a transfer receipt.

## Symptom: yesterday's passing plan no longer applies

**Discriminating evidence:** `retention-gc-apply-plan-drift` compares the original plan reference with a recomputed plan. Other apply diagnostics distinguish a nonpassing original plan, a nonpassing recomputation, and failed admission. Inspect those separately rather than treating every denial as stale bytes.

**Safe next action:** preserve both plan references and the apply record. Inspect changed pins and supplied admission state, then ask the planning owner to reassess against current observations. Replanning is a new decision with its own inputs, not permission to ignore drift.

**Stop condition:** the cause of divergence is not understood or admission is stale, revoked, mismatched, or unavailable. Do not edit the stored plan or force its old result onto current state.

## Symptom: a receipt or tombstone exists, but completion is disputed

**Discriminating evidence:** the evaluator writes receipts and can create a tombstone without performing an object-store deletion. The lifecycle evaluator separately requires plan, apply, execution, and audit, checks their decisions, and checks references and scope across them.

**Safe next action:** identify the actual owning subsystem's effect boundary and inspect its execution evidence, then the corresponding audit. If only the simple fixture ran, report its diagnostic scope honestly; there is no physical-erasure result to recover from those artifacts.

**Stop condition:** execution is unknown or the lifecycle links mismatch. Do not rerun an uncertain effect just to produce a missing record.

## Worked case: profile permission conflicts with action denial

The checked-in fixture profile sets `can_compact: true` for `private-secret-ref`, while the evaluation diagnostic explicitly denies `private-secret-ref` plus action `compact`. This is a source-level boundary observation, not a reproduced runtime bug. A profile field is not evidence that the evaluator consulted or accepted it for that action.

For such a request, retain the denial, identify the class/action pair, and seek a policy/implementation review rather than changing the class to get past the check. Similarly, `legal-hold-class-not-deletable` is an intentional class-level destructive blocker, not missing disk cleanup. The [benchmark rail](../../world-benchmark-sharing-and-retention.md) cannot resolve either issue: its counts and receipts are not deletion authority.

## Sources

- [Handbook](../README.md)
- [World distribution contract](../../world-distribution.md)
- [Technical companion](../../technical/world-effects/distribution-retention-and-reachability.md)
- [Evaluation and fixture diagnostics](../../../src/retention/parts/mod/p025/body.rs)
- [Explanation diagnostics](../../../src/retention/parts/mod/p018/body.rs)
- [Apply drift checks](../../../src/retention/parts/mod/p014/body.rs)
- [Clearance regression cases](../../../src/retention/parts/mod/tests/m000/p001/body.rs)
- [Lifecycle evaluation](../../../src/retention/parts/mod/p017/body.rs)
