# Design: Host admitted recovery groups in the public node

## Ownership

The node shell owns durable orchestration and effect ordering. `molten-core::system_extension` owns group validation, restart planning, lifecycle fencing, migration compatibility and delivery admission. Native `Supervisor` owns actual materialized callback execution, instance journaling and uncertainty. `molten-node-host` supplies capability roots and shared errors, not service semantics. The composition harness observes these owners; it does not implement them.

## Admission and runtime ports

A host is constructed with a ledger capability, a current-authority port and a lifecycle runtime. Every mutating operation asks the authority port for an admitted canonical receipt before changing state. The production runtime contains actual admitted native supervisors and exact pre-admitted replacement bundles; status never constructs a fixture engine. Public CLI input selects admitted artifacts, not permission booleans. The startup source gate must succeed before hosting effects.

## Persistence and process loss

A ledger-rooted Redb database stores canonical Preserves host state. A transaction atomically publishes group history, affected member fences, durable work/current claims and pending lifecycle actions. Redb owns the process-exclusion lock. Finite groups, work and history bound recovery reads. Every lifecycle action is persisted as an intent before invoking the runtime; its observed native instance is persisted before advancing. Process death between intent and observation leaves uncertainty, never permission to repeat a callback. Reconciliation reads the canonical native journal and requires the intended generation/incarnation/phase postcondition. Exact equality with the full pre-action durable instance proves no native callback intent was committed: the native executor durably mutates that instance before every callback effect. Only this proven-not-started case can begin a fresh attempt; a committed callback intent or ambiguous observation remains blocked. A sibling outside the group is not fenced by a group's pending intent.

Restart planning reuses `plan_recovery`, retaining history before stop/restart. Dependent stops run in reverse order and restart runs in declaration order. Replacement does not reset history. Logical ticks are explicit admitted observations, monotonic within a group, never inferred from wall time.

## Durable work

Logical work retains its original generation/incarnation. A current claim is generated only after explicit readmission, persisted before delivery, and revoked when its member is fenced. Delivery checks the exact stored current claim before `admit_work_delivery`; a valid-looking historical claim is insufficient. Unknown native effects are not blindly replayed. Old-generation work is rejected unless admitted transformation explicitly installs replacement work.

## Upgrade

Record drain, checkpoint and replacement intents separately. Replacement uses the owning lifecycle migration admission and actual native recovery callback with exact executable/profile/schema and checkpoint binding. The runtime must materialize and publish transformed state; a migration receipt alone is not transformation evidence. Activation publishes the new generation only after successful callback observation. Failed/interrupted activation stays fenced and operator-visible. The independent sibling is outside the affected action list.

## Evidence

The public-node twin uses real process termination, reopens these stores and projects canonical observations into the existing composition vocabulary. Removing stale-delivery checks, forgetting storm history and skipping transformation must fail corresponding properties. No validation task is complete without a recorded run. This change does not weaken startup admission or grant authority from artifact possession.
