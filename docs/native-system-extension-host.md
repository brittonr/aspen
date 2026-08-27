# Native system-extension host

r[impl molten.system_extension.native_host.profile] r[impl molten.system_extension.native_host.callback_protocol] r[impl molten.system_extension.native_host.executable] r[impl molten.system_extension.native_host.execution] r[impl molten.system_extension.native_host.durability] r[impl molten.system_extension.native_host.intent] r[impl molten.system_extension.native_host.effects] r[impl molten.system_extension.native_host.effect_completion] r[impl molten.system_extension.native_host.ingress] r[impl molten.system_extension.native_host.recovery] r[impl molten.system_extension.native_host.operator] r[impl molten.system_extension.native_host.neutrality] r[impl molten.system_extension.native_host.validation] r[impl molten.system_extension.native_host.nonclaims]

Molten can host an admitted system extension as one bounded process per callback. The host remains workload-neutral.

## Profile and executable admission

The initial profile is `native-host-local-pilot-v1`. It uses:

- execution profile `native-process`;
- ALPN `molten/system-extension/native/v1`;
- framing `preserves-packed-single-frame-v1`;
- a cleared process environment;
- one process for each callback;
- named callback, diagnostic, instance, operation, port, and policy bounds.

Installation binds exact executable bytes, artifact kind, target, dependency closure, materialization, provenance, source gate, policy, authority, resources, state schema, manifest, and execution profile.

A path or executable file does not grant authority. Missing evidence denies before instance publication or process start.

## Callback protocol

The host writes one canonical packed Preserves envelope to standard input. The child writes one canonical packed Preserves outcome to standard output.

The envelope binds:

- manifest, executable, instance, extension, and service identities;
- generation, callback kind, sequence, event, payload, and deadline;
- state, policy, resource, and port references;
- the exact framing profile.

The outcome binds output references, typed effect requests, state, checkpoint, and health. The decoder rejects malformed, non-canonical, trailing, oversized, ambient-effect, and unsupported-schema output.

`molten-native-extension-fixture` is an independent callback producer. The host decoder is the independent consumer.

## Bounded execution

`NativeProcessSystemExtensionExecutor` invokes callbacks only through `ExecutionFabricPort`. It does not call `std::process::Command`.

The executor persists callback intent before it calls the execution port. It records definite pre-start failure, terminal completion, or unknown completion without automatic retry.

The process adapter clears the environment and applies exact input, output, timeout, polling, teardown, and process-group bounds.

## Durable state and recovery

The native instance record contains:

- manifest, executable, profile, and state-schema references;
- lifecycle state and active generation;
- resource use, callback sequence, and event sequence;
- checkpoint reference;
- unresolved and completed operations;
- evidence references and ingress state.

`DurableNativeHostJournal` stores canonical instance records through the existing Redb durability adapter. Restart reloads the latest exact instance record.

Startup classifies unresolved work as not started, running observed, terminal, unknown, or stale. Unknown work remains reconciliation-required and does not retry automatically.

A recovered running instance first records host loss. It then enters the existing bounded restart and checkpoint recovery flow.

## Effects

The callback outcome is validated before any effect is visible. Each effect uses one active manifest binding.

The service persists effect intent before adapter routing. A lost result remains unknown.

A terminal effect observation returns through a generation-fenced `message` callback. The extension decides its semantic transition. The host does not infer workload success.

## Ingress

`NativeServiceIngressPort` accepts only the exact endpoint, peer, service, manifest, generation, authority, policy, resource, transport, ALPN, framing, and payload cohort.

The host commits ingress intent before it returns a service acknowledgement. Transport delivery does not prove callback acceptance.

`NativeServiceClient` is the local pilot client. A future Iroh adapter can implement the same application-owned port.

## Operator workflow

The service shell exposes install, start, request, status, checkpoint, restart, recover, drain, stop, and remove operations.

Drain stops new ingress before the callback. Removal fails when ingress, resource use, unresolved work, or terminal lifecycle evidence is incomplete.

Status uses claim level `local-live-pilot`. It reports canonical operator state and unresolved recovery classes. It omits host paths, process identifiers, backend handles, environment values, and secrets.

## Offline evidence

The artifact index links executable, callback envelope, callback receipt, execution receipt, instance state, effect, checkpoint, and lifecycle evidence.

Offline verification rejects duplicate members, malformed references, missing roles, broken parent links, identity drift, and missing non-claims.

The separate-process integration test runs install, activation, accepted and rejected ingress, effect routing, completion callback, checkpoint, restart recovery, drain, stop, removal, and tamper denial.

## Non-claims

Local pilot evidence does not prove:

- sandboxing or hermeticity;
- executable trust or callback correctness;
- effect success or transport delivery;
- distributed availability;
- production readiness.
