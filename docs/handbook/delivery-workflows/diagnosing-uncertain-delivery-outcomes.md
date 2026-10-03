# Diagnosing uncertain delivery outcomes

Mode: Troubleshooting

Start by naming the uncertain fact: diagnostic-store update, coordination commit, timer scheduling, status publication, or worker external effect. These have different owners and different reconciliation rules. “Delivery failed” is not precise enough to justify replay. Return to the [Handbook](../README.md); the [recovery companion](../../technical/replication/dead-letter-redrive-and-recovery.md) explains the state-machine background.

This procedure is source-checked and has not been executed. It uses existing artifacts and the owning integration's observations; it does not invent a live delivery administration CLI. Preserve the request, policy and manifest references, expected state and revision, planned transition, receipt, commit observation, and timer/status outcomes before choosing an action.

## Symptom: the diagnostic returned `first`, but completion is absent

**Discriminating evidence.** The [first-decision helper](../../../src/delivery/parts/idempotency/p006/body.rs) stores its receipt, window, and dedup entry before returning permission to the caller. The diagnostic does not execute the business effect. A `semantic-result` reference supplied to it is a binding, not a measurement of what subsequently happened.

**Safe next action.** Inspect the caller's actual effect and durable semantic-result evidence separately. Record whether the effect was never started, was observed terminally, or may have happened without terminal evidence. Do not interpret a later `duplicate` as proof that the first external effect succeeded.

**Stop condition.** If external completion remains uncertain, do not manufacture a new producer, scope, sequence, or payload just to obtain another `first`. Such a change evades the comparison rather than resolves the effect.

## Symptom: an apparently identical request is `conflict` or `stale`

**Discriminating evidence.** In the [decision law](../../../src/delivery/parts/idempotency/p001/body.rs), an existing dedup entry is compared first. Exact duplicate requires matching operation reference, payload reference, and evidence vector. Without a matching entry, a sequence below the scope window is stale. Evidence order participates in vector equality; “same collection” is not automatically the same input.

**Safe next action.** Compare canonical operation and entry records, not only human summaries. Check producer, consumer, intent, policy references, scope resolution, and ordered evidence. Preserve the original decision. A changed payload on the same lookup coordinates is a conflict, while another producer at an old sequence can be stale against the shared scope window.

**Stop condition.** Do not delete the store or advance identifiers to silence the classification. Escalate an unexplained binding difference to the producer/integration owner.

## Symptom: a scope artifact disagrees with the name-derived scope

**Discriminating evidence.** The [scope constructor](../../../src/delivery/parts/idempotency/p000/body.rs) hashes a scope profile with an empty retention list when resolving by name. The [scope CLI handler](../../../src/cli/workflow/delivery/ops.rs) can construct an artifact with supplied retention references. Those are different canonical values when the list differs. Explicit scope reference also wins over a supplied scope name.

**Safe next action.** Record which canonical scope was actually used and keep subsequent analysis on that binding. This is a source-review observation relevant to the README's diagnostic examples, not a reproduced failure. The examples' abbreviated references are not current evidence to execute unchanged.

**Stop condition.** Do not merge windows or infer duplicate equivalence from a shared display name.

## Symptom: the coordination receipt is `unknown`

**Discriminating evidence.** The [shell](../../../src/coordination_delivery/service.rs) reads back once after an unknown commit disposition or outcome-unknown error. Exact planned published state yields `applied-after-reconciliation`; exact expected state yields `not-applied-after-reconciliation`; anything else remains `unknown`. A readback error instead leaves a service error, not a fabricated classification.

**Safe next action.** Retain both candidate states and the observed state. Keep currentness, durability, and engine epoch alongside the status: reconciliation retains an original observation when present. An `after-state-ref` in the receipt names the planned transition and cannot independently prove publication.

**Stop condition.** Neither a third state nor absent readback is permission for another compare-and-commit. Do not replay an external worker effect on the strength of a queue-store classification.

## Symptom: durable claim exists but timer or status work failed

**Discriminating evidence.** Timer and status calls occur after a confirmed commit classification. Inspect failed timer references and the full timer/status observations, including their uncertainty Booleans. The canonical receipt does not serialize every observation field.

**Safe next action.** Keep the durable claim and its exact token in view while the responsible port owner reconciles follow-up work. A missing status reference is not an absent claim. A timer failure is not authority to expire work using wall time.

**Stop condition.** Do not resubmit the whole transition to repair a timer. Expiry still requires admitted logical time and the policy's expiry authority.

## Worked failure boundary

The checked-in [uncertainty fixtures](../../../src/coordination_delivery/tests/uncertain.rs) distinguish unknown-before apply, unknown-after apply, and timer failure after commit. In the timer-failure fixture, the expected status is `Applied`, a failed timer reference is recorded, and the stored head matches the planned after-state. Those assertions are inspected source, not results from this documentation work.

If that claimed worker later produces an uncertain external effect, this fixture supplies no exactly-once guarantee. The [addressable actor contract](../../addressable-actor-runtime.md#unknown-effects) requires degraded unknown state and explicit resolution before checkpoint recovery; resolution itself does not authorize automatic effect retry. End diagnosis with the exact unresolved boundary and responsible evidence owner, not a blanket “safe to retry.”

## Sources

- [Handbook](../README.md)
- [Coordination delivery contract](../../coordination-delivery.md)
- [Recovery companion](../../technical/replication/dead-letter-redrive-and-recovery.md)
- [Addressable actor unknown-effect contract](../../addressable-actor-runtime.md)
- [Unknown-commit and timer-failure fixtures](../../../src/coordination_delivery/tests/uncertain.rs)
- [Shell readback and effect ordering](../../../src/coordination_delivery/service.rs)
