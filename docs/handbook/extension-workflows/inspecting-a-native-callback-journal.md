# Inspecting a native callback journal

Mode: How-to

## Goal and prerequisites

Build an evidence-based account of one native callback without confusing process exit, value publication, effect execution, and completion delivery. The output should be a small investigation record identifying the instance, active generation, relevant operation references, last definite observations, and unresolved questions.

You need authorized access to an already configured journal adapter or a preserved canonical instance-record export, the admitted executable/profile/manifest context, and any associated execution and publication evidence. Do not guess a database location or open a live store through an unrelated writer. The [native host contract](../../native-system-extension-host.md) defines the local pilot boundary; [Native intent and recovery](../../technical/extensions/native-intent-and-recovery.md) explains why uncertainty is retained. See the [Handbook](../README.md) for related workflows.

**Status:** this is a source-checked API inspection procedure, not an executed journal session. There is no native-journal inspection subcommand in the inspected `system-extension` CLI: it declares only `run-fixture` and `show`. `show` parses generic operator status, not a Redb database or native instance record.

## 1. Decide which artifact you actually have

If you have canonical packed instance bytes, use the existing `decode_native_instance_record` boundary in an authorized application integration. It performs strict canonical decoding and expects the native instance schema. Do not treat a text rendering, callback frame, or generic status artifact as interchangeable input.

If you have a configured `NativeHostJournal`, its read operations are `latest_instance(instance_id)` and `history(instance_id)`. The durable implementation reads decoded records from the adapter's durable log; the in-memory implementation reads its vector. Neither interface is an instruction to invent adapter authority from a filesystem path.

Stop if decoding or storage fails. Preserve the original bytes and error classification instead of editing the record until it parses. A malformed durable-log record can cause history retrieval to fail rather than silently disappear from the result.

## 2. Anchor the latest record to admitted identities

Compare `instance_id`, extension/service identifiers, `manifest_ref`, `executable_ref`, `profile_ref`, and `state_schema_ref` to independently supplied admission context. Record lifecycle phase and generation before comparing any callback.

`admit_native_instance_recovery` checks identity consistency and continuity prerequisites; it is not a repair function. Running, failed, and restarting records require checkpoint state, and a running record also requires semantic state. `state_ref` and `checkpoint_ref` serve different purposes. Finding one does not discharge the obligation to locate the other.

If identities disagree, stop the recovery proposal. Do not substitute the currently available executable or newest schema merely because its name looks right.

## 3. Follow one operation through history

Select the callback's `operation_ref` from evidence, not its position in a vector. For each relevant history record, note its generation, operation kind, state, parent reference, and terminal reference. Track related `ValuePublication` entries by their parent references. Also distinguish `completed_operations` from `completed_operation_refs`: the former contains terminal records; the latter participates in completion-consumption bookkeeping.

In the executor, callback intent is saved before materializing inputs or calling `ExecutionFabricPort`. Returned values are decoded and admitted before publication. Each publication commits its own intent before calling the value adapter. The durable journal appends canonical bytes with requested `MachineLoss` durability; this does not prove the separate value adapter provides durable publication.

A useful investigation row therefore names the boundary: “callback process observation,” “effect-body publication,” or “provider completion delivery,” rather than just “request succeeded.”

## 4. Classify unresolved work without authorizing retries

Use `classify_native_recovery` against the exact record. Its classification is deterministic:

| Recorded condition | Recovery classification | Inspection consequence |
| --- | --- | --- |
| Generation differs from active generation | `Stale` | Do not deliver it into the current generation |
| `IntentCommitted` | `NotStarted` | Seek subsequent observations; the name is not proof of non-execution |
| `Started` | `RunningObserved` | Preserve the observed execution boundary |
| `Terminal` | `Terminal` | Locate the terminal evidence |
| `Unknown` | `Unknown` | Reconciliation remains required |

Every returned inventory entry has retry permission disabled. A journal classification is not a retry policy, and a canonical receipt cannot mint authority to repeat work.

## 5. Work the publication-loss case

The checked-in value-port test publishes an output under `UnknownAfterAcceptance`: publication reports uncertainty even though the in-memory adapter contains the bytes. This is a precise counterexample to treating “error” as “nothing happened.”

In the executor, an uncertain publication leaves its operation unknown and propagates uncertainty to callback completion. Finding matching bytes later can establish byte identity, but not whether every dependent provider action occurred. Record the known publication identity, the absent definite acceptance observation, and which downstream action remains blocked. Do not rerun the callback or route its effect merely to obtain a cleaner receipt.

The integration tests separately demonstrate that missing, mismatched, or oversized provider output blocks completion delivery while the provider effect remains terminal. Keep that case separate from uncertain provider execution.

## 6. Close with a bounded disposition

Finish with one of: identities and observations consistent; additional named evidence required; or invalid record/context requiring owner review. Include the unresolved operation references and exact boundary needing reconciliation. Do not erase history, remove state, or infer exactly-once behavior. Removal itself requires idle resources, stopped ingress, terminal lifecycle, and no unresolved work; investigation should preserve those blockers, not bypass them.

## Sources

- [Handbook](../README.md)
- [Native system-extension host](../../native-system-extension-host.md)
- [Native intent and recovery](../../technical/extensions/native-intent-and-recovery.md)
- [Journal interface, decoder, and Redb append](../../../src/system_extension/native_host/parts/journal/p000/body.rs)
- [Recovery classification and removal laws](../../../crates/molten-core/src/system_extension/native_host/recovery.rs)
- [Intent and publication ordering](../../../src/system_extension/native_host/parts/executor/p001/body.rs)
- [Result admission and uncertainty propagation](../../../src/system_extension/native_host/parts/executor/p002/body.rs)
- [Value-port uncertainty and journal tests](../../../src/system_extension/native_host/tests.rs)
- [Provider-output denial cases](../../../tests/parts/nativesystemextension/p001/body.rs)
- [Actual CLI surface](../../../src/cli/runtime/system_extension/command.rs)
