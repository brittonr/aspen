# Diagnosing blocked session drains

Mode: Troubleshooting

A blocked `drain-sessions` task should first be treated as an evidence or binding problem, not as a request to force cutover. The [README requirement](../../../README.md#upgrade-session-protocol-drains) requires ledger-resolved, passing protocol lifecycle evidence for the affected old protocol. This guide separates specific denial causes and safe next actions. It is source-checked and was not executed against a running session.

Preserve the exact plan, task ID, ledger root, upgrade store, denied receipt, and current name-pointer evidence before considering changes. The protocol gate, upgrade receipt, and completion status are different objects. Do not substitute a terminal log or successful CLI exit for their decisions.

## Establish whether the task ran

Use existing status before attempting another stateful action. This read-only invocation is supported by the [root group](../../../src/main/root/parts/command/p000/body.rs), [upgrade declaration](../../../src/cli/runtime/upgrade/command.rs), and [status implementation](../../../src/upgrades/parts/mod/p003/body.rs). Source-checked; not executed:

```sh
molten test upgrade status \
  --store "${UPGRADE_STORE:?Set the existing upgrade store}" \
  --plan-ref "${UPGRADE_PLAN_REF:?Set the exact reviewed plan reference}"
```

If an earlier task is incomplete, normal execution rejects out-of-order tasks before evaluating their bodies. Cutover has a special denial-receipt path for incomplete predecessors. Diagnose the first missing completion, not just the last requested task. A malformed status reference can fail parsing; a missing or mismatched stored receipt is not accepted as completion. Never repair this by writing a checkbox into `status`.

## Symptom: evidence is not readable from the ledger

**Discriminating evidence:** The [drain shell](../../../src/upgrades/parts/mod/p008/body.rs) reports that a specific evidence reference is not readable. It unions and deduplicates the task's precondition and postcondition refs, then reads each from the supplied ledger.

**Safe next action:** Compare the ledger root used for this session with the provenance of the gate artifact. Determine whether the gate was produced but never made available in that ledger, or whether the plan names a different artifact. Keep the missing reference in the incident record. Merely placing a receipt in the upgrade store's `receipts` directory does not satisfy the separate ledger lookup.

**Stop condition:** Until the exact gate bytes are available and verified through the normal evidence workflow, leave the task incomplete. A syntactically valid content reference is not an available artifact.

## Symptom: evidence is not a protocol-session gate

**Discriminating evidence:** The ledger lookup succeeds but `parse_protocol_session_gate_receipt` fails. A generic upgrade receipt, transcript receipt, or unrelated review artifact cannot stand in for `protocol-session-gate-receipt-v1`.

**Safe next action:** Identify the actual record kind and its producer. Audit every task evidence reference, because this path tries each as a protocol gate. A valid gate plus an unrelated evidence item still accumulates diagnostics and denies.

**Stop condition:** Do not discard an inconvenient reference merely to obtain a pass. Correct the reviewed plan's evidence classification and preserve the original plan identity and denial history.

## Symptom: gate denies or lacks terminal state

**Discriminating evidence:** The receipt's decision is not `pass`, or it lacks session IDs or terminal-state refs. The upgrade evaluator requires both collections to be nonempty. Its diagnostic distinguishes a denied gate from one that does not bind terminal session state.

**Safe next action:** Return to the protocol lifecycle producer. Inspect the sessions and terminal evidence that the gate actually covers. Obtain a new justified observation only through that producer's workflow; do not change the gate decision text or manufacture terminal refs.

**Stop condition:** A terminal state reference is evidence about the represented lifecycle, not proof that every remote participant is currently quiescent. Uncertain live effects require protocol-owner investigation, not unconditional retries.

## Symptom: wrong protocol or stale compatibility

**Discriminating evidence:** “Expected one of” identifies a protocol mismatch; “stale compatibility ref” identifies from/to references absent from the corresponding compatibility sets. Explicit from/to refs must also belong to the plan's affected set.

**Safe next action:** Compare exact content identities. The [binding selector](../../../src/upgrades/parts/mod/p003/body.rs) prioritizes `from_ref`, then a canonical `subject`, then old compatibility refs, and finally affected refs. Do not substitute the new protocol's successful gate for evidence about draining the old protocol.

**Stop condition:** The inspected “stale” check is a set-binding comparison. It is not a wall-clock expiry check. Resolve age or release-candidate freshness under the applicable policy separately.

## Worked failure: passing gate, wrong old artifact

The [negative drain fixture](../../../src/upgrades/parts/mod/tests/m000/p002/body.rs) imports a passing gate, then builds a plan whose old protocol is a different generated fixture reference. Execution returns deny and the fixture checks for the mismatch diagnostic. Another case changes only compatibility old refs and expects the stale-binding diagnostic. These are checked-in cases, not test results from this documentation work.

The same file checks that denied drain and cutover attempts preserve the snapshot and routing pointer. The snapshot covers `plans`, `names`, and `status`; diagnostic receipt writes are outside it. Thus “no mutation on deny” must not be paraphrased as “no filesystem write anywhere” or as crash-atomic rollback of external effects.

If a denial claims a failed no-mutation check, preserve all evidence and stop further mutation. The separate [system-extension migration theory](../../technical/extensions/upgrade-quarantine-and-migration.md) explains why post-replacement failures cannot be treated like precondition denial. Neither path grants authority or automatic recovery.

## Sources

- [Handbook](../README.md)
- [Upgrade drain requirement](../../../README.md#upgrade-session-protocol-drains)
- [Architecture](../../architecture.md)
- [Upgrade and migration theory](../../technical/extensions/upgrade-quarantine-and-migration.md)
- [Drain shell and predicates](../../../src/upgrades/parts/mod/p008/body.rs)
- [Negative drain fixtures](../../../src/upgrades/parts/mod/tests/m000/p002/body.rs)
- [Status receipt validation](../../../src/upgrades/parts/mod/p010/body.rs)
