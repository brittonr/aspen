# Following an upgrade plan

Mode: Walkthrough

This source-only walkthrough follows `name_move_session_keeps_artifacts_immutable_and_receipted`, a checked-in Rust fixture, from imported values to a moved name and denied cleanup. It is useful when reviewing what an upgrade session actually changes. No commands or tests were executed for this article. The fixture uses generated test references and synthetic source-gate evidence; it is not a deployment recipe.

The [architecture](../../architecture.md) supplies the identity and authority rules. The [technical companion on upgrade and migration](../../technical/extensions/upgrade-quarantine-and-migration.md) explains a different boundary: replacing a system-extension generation. Do not treat a successful metadata session as proof that an executor recovered state.

## 1. Identify the inputs before considering the plan

Open the [name-move fixture](../../../src/upgrades/parts/mod/tests/m000/p000/body.rs). Its isolated root contains separate `ledger` and `upgrades` directories. The ledger imports the Preserves values `<module "old">` and `<module "new">`, then a `dependent` record containing the old artifact reference and the string `uses old`.

These are real fixture values, not invented content-reference strings. Their canonical Preserves hashes determine their identities. The friendly name `app/main` is metadata that will point at one identity; it does not rename or rewrite the old module bytes.

Record three inputs in review notes: old artifact, new artifact, and dependent artifact. Also record which ledger supplied them. A matching-looking name in another store is not evidence that this session has the same dependency population.

## 2. Follow impact discovery and task construction

The fixture calls `name_move_plan_value` with session ID `session-name-move`, the name, both artifact references, and test initiator, capability, policy, and transcript references. Its `source_gate_values` helper supplies synthetic test evidence. Actual plan construction validates source-gate receipt values; these fixture inputs must not be promoted to current release approval.

The [planner](../../../src/upgrades/parts/mod/p001/body.rs) selects ledger impact discovery when no registry root is supplied. A separate fixture in the same test file exercises registry reverse dependencies. In this walkthrough, the explicit assertions require both the old artifact and the dependent in `impact_refs`. They do not establish that every external consumer has been discovered.

The generated task order is:

1. `compatibility-alias`, targeting `app/main@candidate`;
2. `transcript-gate`, whose kind is `transcript-rerun`;
3. `move-name`, targeting `app/main`;
4. `cutover`, also targeting `app/main`.

The plan records old/new compatibility sets, policy references, and the old artifact as a rollback reference. Its compatibility expiry is absent. Review that order rather than assuming the word “cutover” is the first state-changing boundary.

## 3. Separate session creation from execution

`create_session` parses and stores the plan and emits a `session-create` receipt. It requires capability and policy references, but their presence here is not a new grant of authority. The fixture then initializes `app/main` to the old artifact with `set_name_pointer`.

Observable boundaries are the stored plan, creation receipt, and name-pointer artifact. The store uses `plans`, `receipts`, `names`, and `status` areas. Content-derived filenames and status keys are implementation details; the references inside the canonical values are the evidence to retain.

## 4. Follow effects task by task

The fixture executes the four task IDs in order and asserts a passing receipt for each. The [task dispatcher](../../../src/upgrades/parts/mod/p007/body.rs) writes the candidate alias, checks that the transcript task has nonempty precondition references, and delegates the name move. Importantly, that transcript check does not execute a transcript or resolve a passing replay receipt.

The [name-move implementation](../../../src/upgrades/parts/mod/p002/body.rs) rejects an existing pointer that differs from `from_ref`; otherwise it writes the new pointer. This occurs before the final cutover task. The [cutover evaluator](../../../src/upgrades/parts/mod/p004/body.rs) checks readiness and completion evidence; it does not itself invoke a live replacement adapter.

For an interrupted real operation, therefore, inspect both pointer state and task receipts. “Cutover incomplete” does not necessarily mean “name unchanged.” Nor does an error after a write justify an unconditional retry.

## 5. Read the end state and its limits

The fixture reads `app/main`, asserts that it points to the new artifact, and checks that no task IDs remain. Completion is backed by [stored-receipt validation](../../../src/upgrades/parts/mod/p010/body.rs): the receipt must hash to its reference, pass, and match the plan and task. A handwritten checkbox does not satisfy this boundary.

Finally, cleanup admission for the old artifact returns deny. Successful movement did not erase dependency or retention obligations. Preserve that denial as part of the example rather than “fixing” it by deleting old state.

The available path proves source-level fixture intent for metadata movement and receipt bookkeeping. Live protocol draining, application migration, release authority, and system-extension recovery require their own evidence. None was exercised while preparing this walkthrough.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Upgrade and migration theory](../../technical/extensions/upgrade-quarantine-and-migration.md)
- [Name-move and rollback fixtures](../../../src/upgrades/parts/mod/tests/m000/p000/body.rs)
- [Plan and session implementation](../../../src/upgrades/parts/mod/p001/body.rs)
- [Task effects and rollback](../../../src/upgrades/parts/mod/p002/body.rs)
- [Receipt-backed status](../../../src/upgrades/parts/mod/p010/body.rs)
