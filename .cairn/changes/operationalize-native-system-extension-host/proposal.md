# Proposal: Operationalize the native system-extension host

## Why

Molten has a `SystemExtensionHost`, canonical manifests, lifecycle transitions, callback admission, effect routing, resource accounting, and fixture executors.

The operator command supports only `system-extension run-fixture` and `show`. No node service can install and supervise a real native-process extension from an admitted executable artifact.

A production-shaped native-process profile is required for application-owned services that cannot run as no-WASI components. Kiln is the first planned consumer, but the host must remain workload-neutral.

## What Changes

- Add a concrete native-process `SystemExtensionExecutor` over the accepted bounded execution fabric port.
- Define one canonical callback framing protocol for request and response bytes.
- Invoke one bounded process per callback with a cleared environment and explicit artifact, workspace, input, output, deadline, and teardown bindings.
- Add durable extension-instance, callback-intent, effect-intent, checkpoint, recovery, and terminal-state records.
- Add node composition for install, activate, request, message, timer, health, checkpoint, recover, drain, shutdown, status, and removal workflows.
- Route callback-requested effects only through exact active-generation fabric-port bindings.
- Add versioned transport ingress for an installed service without exposing backend handles to the extension.
- Add operator commands and canonical receipts for service installation, startup, status, recovery, drain, shutdown, and failure.
- Add positive and negative separate-process, restart, stale-generation, malformed-callback, timeout, cancellation, output-bound, and effect-denial tests.
- Keep the initial native-process profile explicitly non-production until its own promotion gate passes.

## Dependencies

This change depends on the active `add-bounded-execution-fabric-port` change.

The native executor must consume that accepted port. It must not add a second direct process shell.

## Impact

- **System extension**: concrete native-process execution and durable service operation.
- **Node runtime**: extension instance registry, service lifecycle composition, ingress, and recovery.
- **Durability**: callback and effect intent, checkpoint, generation, and unresolved-operation records.
- **Transport**: exact service protocol registration and bounded callback ingress.
- **CLI**: install, start, request, status, recover, drain, stop, and removal workflows.
- **Testing**: pure, shell, separate-process, restart, failure, and offline receipt verification.

## Non-goals

- Define Kiln, database, queue, scheduler, or workflow semantics in Molten core.
- Load arbitrary unreviewed native executables.
- Grant ambient filesystem, environment, network, process, device, or credential authority.
- Claim that native execution is sandboxed or hermetic.
- Add automatic callback or effect retries after uncertain outcomes.
- Promote the initial profile to production readiness.
