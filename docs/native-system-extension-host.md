# Native system-extension host

r[impl molten.system_extension.native_host.profile] r[impl molten.system_extension.native_host.callback_protocol] r[impl molten.system_extension.native_host.executable] r[impl molten.system_extension.native_host.execution] r[impl molten.system_extension.native_host.durability] r[impl molten.system_extension.native_host.intent] r[impl molten.system_extension.native_host.effects] r[impl molten.system_extension.native_host.effect_completion] r[impl molten.system_extension.native_host.effect_completion_value] r[impl molten.system_extension.native_host.ingress] r[impl molten.system_extension.native_host.recovery] r[impl molten.system_extension.native_host.operator] r[impl molten.system_extension.native_host.neutrality] r[impl molten.system_extension.native_host.validation] r[impl molten.system_extension.native_host.nonclaims] r[impl molten.system_extension.native_host.value_protocol] r[impl molten.system_extension.native_host.value_materialization] r[impl molten.system_extension.native_host.value_publication] r[impl molten.system_extension.native_host.value_intent] r[impl molten.system_extension.native_host.semantic_state] r[impl molten.system_extension.native_host.value_validation]

Molten can host an admitted system extension as one bounded process per callback. The host remains workload-neutral.

## Profile and executable admission

The materializing profile is `native-host-local-pilot-v2`. It uses:

- execution profile `native-process`;
- ALPN `molten/system-extension/native/v2`;
- framing `preserves-packed-materialized-values-v2`;
- a cleared process environment;
- one process for each callback;
- named callback, value, diagnostic, instance, operation, port, and policy bounds;
- mandatory materialized values without a v1 fallback.

Installation binds exact executable bytes, artifact kind, target, dependency closure, materialization, provenance, source gate, policy, authority, resources, state schema, manifest, and execution profile.

A path or executable file does not grant authority. Missing evidence denies before instance publication or process start.

## Callback protocol

The host writes one canonical packed Preserves envelope to standard input. The child writes one canonical packed Preserves outcome to standard output.

The envelope binds:

- manifest, executable, instance, extension, and service identities;
- generation, callback kind, sequence, event, and deadline;
- exact payload and prior-state reference-and-byte values;
- policy, resource, and port references;
- the exact v2 framing profile.

The outcome binds exact output, effect-request, next-state, and checkpoint values. Each value includes its BLAKE3 reference and bytes.

The decoder rejects missing bytes, identity drift, malformed values, non-canonical frames, trailing data, oversized data, ambient effects, and unsupported schemas.

`molten-native-extension-fixture` is an independent callback producer. The host decoder is the independent consumer.

## Bounded execution

`NativeProcessSystemExtensionExecutor` invokes callbacks only through `ExecutionFabricPort`. It does not call `std::process::Command`.

The executor persists callback intent before value reads or process execution. It materializes payload and state through `NativeCallbackValuePort`.

After execution, it admits all returned values and effect metadata. It persists each publication intent before the value port can publish bytes.

A definite publication rejection becomes terminal. Uncertain publication remains unknown and blocks automatic retry and dependent provider routing.

`InMemoryNativeCallbackValuePort` is a conformance adapter only. A deployment adapter must provide bounded reads, durable exact publication, and explicit acceptance uncertainty.

The process adapter clears the environment and applies exact input, output, timeout, polling, teardown, and process-group bounds.

## Durable state and recovery

The native instance record contains:

- manifest, executable, profile, and state-schema references;
- lifecycle state and active generation;
- resource use, callback sequence, and event sequence;
- latest semantic state and lifecycle checkpoint references;
- unresolved and completed callback, publication, ingress, and effect operations;
- evidence references and ingress state.

`DurableNativeHostJournal` stores canonical instance records through the existing Redb durability adapter. Restart reloads the latest exact instance record.

Startup classifies unresolved work as not started, running observed, terminal, unknown, or stale. Unknown work remains reconciliation-required and does not retry automatically.

A recovered running instance first records host loss. It then enters the existing bounded restart and checkpoint recovery flow.

## Effects

The callback outcome is validated before any effect is visible. Every effect request body publishes before routing. Each effect uses one active manifest binding.

The service persists effect intent before adapter routing. A lost result remains unknown.

A terminal effect observation returns through a generation-fenced `message` callback. Version two binds the exact optional provider output value.

A materializing native profile requires bounded output bytes whose BLAKE3 identity matches the effect output reference. Missing or changed bytes block callback delivery.

The provider effect remains terminal when value admission fails. The host does not retry it or infer workload success. The extension decides its semantic transition.

## Ingress

`NativeServiceIngressPort` accepts only the exact endpoint, peer, service, manifest, generation, authority, policy, resource, transport, ALPN, framing, and payload cohort.

The host commits ingress intent before publishing the exact payload bytes. It dispatches only after identity-checked publication succeeds.

Transport delivery does not prove payload publication or callback acceptance.

`NativeServiceClient` is the local pilot client. A future Iroh adapter can implement the same application-owned port.

## Operator workflow

The service shell exposes install, start, request, status, checkpoint, restart, recover, drain, stop, and remove operations.

Drain stops new ingress before the callback. Removal fails when ingress, resource use, unresolved work, or terminal lifecycle evidence is incomplete.

Status uses claim level `local-live-materialized-values-pilot`. It reports canonical operator state and unresolved recovery classes. It omits host paths, process identifiers, backend handles, environment values, and secrets.

## Offline evidence

The artifact index links executable, callback envelope, callback receipt, execution receipt, instance state, semantic state, value publication, effect, checkpoint, and lifecycle evidence.

Offline verification rejects duplicate members, malformed references, missing roles, broken parent links, identity drift, and missing non-claims.

The separate-process integration test runs install, activation, accepted and rejected ingress, effect routing, completion callback, checkpoint, restart recovery, drain, stop, removal, and tamper denial.

## Non-claims

Local pilot evidence does not prove:

- sandboxing or hermeticity;
- executable trust, value meaning, value durability, or callback correctness;
- effect success or transport delivery;
- distributed availability;
- production readiness.
