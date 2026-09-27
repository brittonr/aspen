# Upgrade, Quarantine, and Migration

Replacing a system extension combines three distinct questions: whether the new generation is legal, whether its manifest accepts the source state schema, and whether its executor actually recovers useful state. This article assumes the admission and lifecycle model in the [system-extension runtime](../../system-extension-runtime.md). It explains the existing host cutover rather than proposing an upgrade orchestrator. See the [Technical companion](../README.md) for the admission and fencing prerequisites.

## Compatibility is directional admission

`plan_state_migration` accepts a destination admitted manifest plus source and target schema names. It checks both names for valid token shape, requires the source schema in the destination's `compatible_state_schemas`, and requires the target to equal the destination's `state_schema`. Its result contains source and target schema names. The function neither reads checkpoint bytes nor transforms application state. This is visible in the [manifest and migration implementation](../../../crates/molten-core/src/system_extension/manifest.rs).

Compatibility is therefore directional. A destination that accepts an old schema does not imply that the old implementation accepts the new schema. Rollback needs its own admissible direction; artifact retention alone does not supply it. Self-compatibility is separately required at manifest admission, ensuring that the current schema is not excluded from the declared accepted set.

This division prevents a common category error: a migration receipt records an admitted schema relationship and cutover context, not proof of a correct data conversion. Recovering code is still responsible for interpreting the supplied checkpoint and producing admitted semantic state.

## Replacement ordering and irreversible observations

The host's `replace_generation` first checks stable extension and service identity, exact executor/manifest execution-profile agreement, a declared `Recover` callback, and a checkpoint reference matching the active lifecycle state. It then plans migration using the destination manifest and computes the next generation with checked arithmetic. See the [generation replacement shell](../../../src/system_extension/parts/host/p003/body.rs).

After these preconditions, the host applies the begin-upgrade or begin-rollback transition, creates and records canonical migration evidence, installs the destination manifest and executor, and invokes recovery using the named checkpoint. Only a successful executed recovery callback is followed by the corresponding success transition and returned replacement artifacts.

This ordering matters on failure. The destination executor is installed before its recovery callback runs. The function is not a transaction that silently restores the old executor when recovery fails. Reviewing only the successful return value would hide the intermediate active generation and failure evidence. Canonical receipts expose boundaries; they do not erase the fact that a later boundary can fail.

## Quarantine is not an unlimited restart loop

The [supervision law](../../../crates/molten-core/src/system_extension/supervision.rs) permits restart only for `Retryable` failure while `restart_attempts < max_restart_attempts`. Fatal, policy, resource, and generation failures quarantine. The lifecycle's failure helper selects `Failed` or `Quarantined` with corresponding health, while `BeginRestart` increments the attempt count with checked arithmetic. A healthy later phase does not, in the inspected transition code, automatically reset that counter.

The [transition relation](../../../crates/molten-core/src/system_extension/lifecycle.rs) allows upgrade from running or drained state. Rollback also admits failed and quarantined states. Both create the next generation; rollback does not decrement the generation to the number formerly used by old code. Quarantine closes normal request dispatch because those callbacks require `Running`, but it is not an assertion that no operator-controlled lifecycle action is possible. Shutdown and an admissible rollback remain explicit paths.

The plugin-specific [lifecycle FSM](../../plugin-lifecycle-fsm.md) also has upgrade and recovery guards, but those guard receipts do not replace system-extension migration admission or generation checks.

## Worked asymmetric rollback scenario

Consider an illustrative ledger extension at generation 12 with schema `ledger-state-v1` and a named checkpoint. A destination manifest declares `ledger-state-v2` and accepts both v1 and v2. Its upgrade can pass the schema relation, advance to generation 13, and invoke recovery. Suppose recovery returns malformed state evidence. The generic host's outcome-denial path classifies invalid outcomes as policy violations, so supervision quarantines rather than repeatedly executing the same invalid recovery.

An operator now supplies the old executable. Calling this action rollback does not restore generation 12. It needs a new admitted destination and generation 14. Moreover, if that destination accepts only v1 while the currently installed manifest's schema is v2, the shell's migration planning rejects the source schema. The fact that the checkpoint originated before the attempted upgrade does not change which schema the inspected replacement code supplies as its source: it uses the active manifest's current schema.

This is an important review boundary, not a new migration guarantee. Operators need to inspect the actual active manifest, checkpoint, and permitted schema relationship rather than infer compatibility from labels such as “previous release.” Nothing in the pure plan proves that a checkpoint's application-level contents satisfy either schema's intended semantics.

## Review and suggested verification

Review precondition denial separately from post-cutover failure. Suggested cases include identity change, executor profile mismatch, missing recovery callback, checkpoint mismatch, unsupported source schema, generation overflow, and destination recovery returning invalid state. Then inspect the resulting phase, active generation, restart count, and evidence rather than just whether the operation returned an error.

The existing [core tests](../../../crates/molten-core/src/system_extension/tests.rs) cover explicit lifecycle and checkpoint/recovery failure paths. The [host implementation](../../../src/system_extension/parts/host/p001/body.rs) distinguishes executor failure from outcome denial, which determines the supervision class. These are source-backed review pointers, not claims that a replacement scenario was executed while writing this article.

## Limits and non-claims

The cutover does not claim atomic migration across providers, automatic rollback, semantic schema equivalence, or uninterrupted distributed service. The generic lifecycle allows upgrade from `Running`; no blanket drain-before-upgrade requirement is inferred here. Native deployment also has the durable intent and reconciliation obligations in the [native host reference](../../native-system-extension-host.md), which are not discharged by an in-memory generation transition. Production readiness requires evidence beyond admission, recovery success, and canonical migration artifacts.

## Sources

- [System-extension runtime](../../system-extension-runtime.md)
- [Native system-extension host](../../native-system-extension-host.md)
- [Plugin lifecycle FSM](../../plugin-lifecycle-fsm.md)
- [State migration admission](../../../crates/molten-core/src/system_extension/manifest.rs)
- [Lifecycle and generation transitions](../../../crates/molten-core/src/system_extension/lifecycle.rs)
- [Failure supervision](../../../crates/molten-core/src/system_extension/supervision.rs)
- [Generation replacement ordering](../../../src/system_extension/parts/host/p003/body.rs)
- [Host outcome failure classification](../../../src/system_extension/parts/host/p001/body.rs)
- [Core lifecycle regression cases](../../../crates/molten-core/src/system_extension/tests.rs)
