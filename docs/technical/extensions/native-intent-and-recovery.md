# Native Intent and Recovery

Native recovery reconstructs what the host knows about interrupted work without treating missing observations as proof of non-execution. This article assumes the materialized-value protocol in the [native system-extension host](../../native-system-extension-host.md). It explains intent ordering, uncertainty, and recovery inventory for that local pilot, rather than designing a new distributed transaction protocol. Related articles appear in the [Technical companion](../README.md).

## The durable object is a host knowledge record

The native instance record binds manifest, executable, profile, and state schema; lifecycle and generation; resource and sequence state; semantic state and checkpoint references; and operation/evidence collections. It is not simply a process identifier to restart. `DurableNativeHostJournal` serializes canonical instance records through the Redb durability adapter, while an in-memory journal supports conformance use. The [journal implementation](../../../src/system_extension/native_host/parts/journal/p000/body.rs) exposes saving, latest-instance lookup, and history.

These records distinguish semantic state from lifecycle checkpoint state. A callback's admitted next-state value is not automatically interchangeable with the checkpoint selected for restart recovery. The [general runtime](../../system-extension-runtime.md) keeps checkpoint and recovery explicit for that reason.

Pure operation-law functions do not persist anything themselves. `commit_native_operation_intent` validates an operation against the active generation, requires `IntentCommitted` with retry disabled, checks duplicate identities against its inspected collections, enforces the unresolved-operation bound, and returns a new instance record. The shell must save that record before crossing the external boundary. The [recovery implementation](../../../crates/molten-core/src/system_extension/native_host/recovery.rs) and executor ordering together establish that division of responsibility.

## Intent before materialization and execution

`invoke_native` commits callback intent before materializing payload and prior-state values. It constructs and independently decodes the canonical callback envelope before invoking `ExecutionFabricPort`. The executor itself routes execution through that port, rather than directly spawning with an ambient process API. These steps are visible in the [native executor](../../../src/system_extension/native_host/parts/executor/p001/body.rs).

The v2 profile requires reference-and-byte values, not reference-only placeholders. A successful process observation is still insufficient: `accept_execution_receipt` requires an accepted exit observation and nontruncated stdout, decodes the bounded outcome, projects it to the generic callback outcome, and validates it before publishing returned values. Publication includes response outputs, effect-request bodies, semantic state, and checkpoint values. See [result admission](../../../src/system_extension/native_host/parts/executor/p002/body.rs).

Each publication has its own committed intent before `NativeCallbackValuePort::publish`. If publication definitely rejects, the operation receives terminal evidence. If the failure reports that publication may have occurred, the operation becomes `Unknown` without inventing a terminal observation. This uncertainty also propagates to the callback completion record. The ordering preserves an important distinction: a process can have exited while the host still lacks a definite publication outcome.

## Recovery classification is not retry authorization

`classify_native_recovery` maps unresolved operation state to `NotStarted`, `RunningObserved`, `Terminal`, `Unknown`, or `Stale`; a generation mismatch takes precedence and yields `Stale`. Every returned inventory entry has `is_retry_permitted: false`. In particular, the name `NotStarted` describes the recorded `IntentCommitted` state. It is not a general theorem that no external work could have occurred across a crash window.

Recovery admission checks that profile, executable, manifest, and state-schema references match the admitted executable context. Running, failed, and restarting records require checkpoint state, and running records require semantic state. These checks establish a coherent recovery input, not recovery success. The native-host governing workflow records host loss for a recovered running instance and then uses bounded restart and checkpoint recovery.

Removal is similarly evidence-sensitive. `admit_native_removal` blocks unresolved operations, active ingress, non-idle resources, and a lifecycle without terminal state. Removing a process or stopping new requests is not enough to erase unresolved external work.

## Worked publication-loss scenario

Suppose, illustratively, a callback returns a next-state value and a storage effect request. The executor validates their bytes and metadata. It commits publication intent for the effect body, calls the value adapter, and then loses a definite result. The adapter reports possible publication.

The host cannot choose between “body absent” and “body published” from that observation. Recording `Unknown` retains the ambiguity. Returning a fresh success receipt would overclaim publication; automatically resubmitting the callback could produce a second semantic transition. Dependent provider routing remains blocked under the native-host contract until uncertainty is reconciled.

Even if an operator can locate bytes with the expected BLAKE3 identity, that observation alone does not prove that every dependent effect was routed or consumed. Value identity, publication acceptance, provider execution, and extension semantics are separate propositions. Recovery evidence needs to answer the particular unresolved question, not merely show that something with a related reference exists.

## Review and suggested verification

Review every external boundary for a preceding saved intent and a subsequent definite or uncertain observation. The [native host tests](../../../src/system_extension/native_host/tests.rs) contain canonical envelope/outcome cases, rejected reference-only values, value-port uncertainty cases, and in-memory/Redb journal round trips. Suggested failure-injection review places interruption between intent save, publication, observation save, and dependent routing. These are suggested checks; this article reports no execution of them.

Also distinguish terminal provider execution from successful completion delivery. The governing native contract keeps a provider effect terminal if output-value admission fails; it does not retry the provider or infer workload success. The extension decides its semantic transition after an admitted completion callback.

## Limits and non-claims

The profile is a local materialized-values pilot. Canonical journal records do not prove durable publication by a deployment value adapter, distributed availability, or executable trust. An in-memory value port is a conformance adapter, not a production durability implementation. Intent identities and completion deduplication do not establish exactly-once external effects. Unknown remains a bounded, explicit knowledge state rather than a retry policy hidden behind an error string.

## Sources

- [Native system-extension host](../../native-system-extension-host.md)
- [System-extension runtime](../../system-extension-runtime.md)
- [Pure native recovery and removal laws](../../../crates/molten-core/src/system_extension/native_host/recovery.rs)
- [Callback and publication intent ordering](../../../src/system_extension/native_host/parts/executor/p001/body.rs)
- [Process result and value admission](../../../src/system_extension/native_host/parts/executor/p002/body.rs)
- [Canonical native journal](../../../src/system_extension/native_host/parts/journal/p000/body.rs)
- [Native protocol and journal tests](../../../src/system_extension/native_host/tests.rs)
