# Aspen Runtime Applications

Status: design note with an active implementation slice. This document captures the target architecture for treating Aspen as a distributed infrastructure runtime and distinguishes the pieces now implemented under OpenSpec change `define-runtime-service-core` from future runtime-service shell work.

Implemented in the current slice:

- `crates/aspen-runtime-core` contains portable, serializable host-loading/model contracts for runtime host kinds, artifacts, service specs, service instances, lifecycle/health state, route declarations, placement hints, restart/upgrade/health policy, capability bindings, and redacted receipts.
- `NativeBuiltIn` services are modeled as linked built-ins through `NativeBuiltInServiceFactory`, `NativeServiceManifest`, and `NativeLoadingPolicy::LinkedBuiltInOnly`; this is intentionally not dynamic native plugin loading.
- `crates/aspen-forge/src/runtime_service.rs` exposes Forge as the first linked native runtime-service wrapper with manifest, route declarations, health receipts, and lifecycle receipts while preserving existing `ForgeNode` internals.
- [`runtime-sponsorship.md`](runtime-sponsorship.md) documents the sponsored runtime grant boundary: Aspen enforces resource authority, placement admission, quota ledgers, and redacted receipts while settlement stays external and currency-neutral.

Still active/future work:

- durable runtime-service reconciler/storage;
- node-local service host/supervision shell;
- route registry integration beyond the current Forge wrapper metadata;
- dynamic non-built-in application installation;
- executioner generalization of jobs/CI.

## Thesis

Aspen already has the substrate for distributed, temporal execution: Raft/KV for durable cluster state, Iroh for transport, capability auth, content-addressed artifacts, jobs/CI executors, plugin manifests, deployment state, and operator receipts. What is missing is a coherent runtime contract for applications above the KV layer.

The target layering is:

```text
Core substrate
  Raft / KV / redb / Iroh / blob / auth / clocks / metrics

Runtime control plane
  applications / services / execution runs / routes / capabilities / receipts

Runtime workers
  native services / execution runners / WASM or Hyperlight guests / Nix-built binaries

Applications
  Forge / executioner policies / docs / cache / hooks / federation / user apps

Extensions and adapters
  app plugins / policy hooks / Git, JJ, HTTP, SSH, Nix, federation bridges
```

Aspen should not make every application a WASM plugin. The runtime contract should support built-in native Rust services, Nix-built binaries, WASM components, Hyperlight guests, OCI artifact ingestion/lowering, and later microVM/unikernel guests. The stable boundary is the declaration of lifecycle, routes, capabilities, state, artifacts, health, and receipts.

## Runtime host-loading taxonomy

Portable host-loading model types live in `crates/aspen-runtime-core`. The crate is data-only: it defines host kinds, artifact profiles, lifecycle state, route ownership, capability bindings, resources, and receipts without starting processes or performing network/filesystem/crypto I/O.

Host kind and artifact identity are intentionally separate:

| Host kind | Artifact profile | Loading rule |
| --- | --- | --- |
| `NativeBuiltIn` | `BuiltIn { name, version }` | First-party services such as Forge are linked into `aspen-node` and registered through a built-in service factory registry, not loaded through `dlopen`-style native plugins. |
| `NativeProcess` | `NativeBinary { hash, store_path, entrypoint }` | Future trusted operator-installed binaries run out-of-process behind an IPC/host-ABI boundary. |
| `Wasm` | `WasmModule { module_hash, abi, entrypoint }` | Modules are fetched by content identity, checked against ABI/resource/capability policy, then instantiated with only scoped host functions. The `WasmRuntimeProfile` records ABI version, runner capability/version, deterministic extension versus service-fragment mode, fuel/memory/time/input/output limits, declared host-function bindings, and redacted output receipts before KV/blob/route/network/clock/secret/log/metrics/timer/output handles are exposed. |
| `Hyperlight` | `HyperlightImage { image_hash, entrypoint }` | Isolated execution runs or later services start only on compatible Hyperlight runners. The `HyperlightRuntimeProfile` records runner capability, runner version, ABI profile, artifact profile, declared host-call bindings, resource policy, and redacted output receipts before KV/blob/logging/metrics/timer/route/output handles are exposed. |
| `OciImage` ingestion/lowering | `OciImage { image_digest, entrypoint, args }` | OCI is a content-addressed packaging/compatibility artifact, not a production host boundary. Production tenant, CI, remote application, and risky adapter admission uses an `OciLoweringPlan` to select an isolated target such as `MicroVm` by default, or `Hyperlight`, `Wasm`, or a VM-backed `Unikernel` profile when a provenance-backed transform/rebuild is compatible. Plain Podman/Docker-style host containers are dev/unsafe-only if ever enabled and are rejected by default production admission. |
| `MicroVm` | `LinuxGuest` or `Unikernel` | Firecracker, Cloud Hypervisor, Uhyve, or QEMU microvm/loader engines require compatible node runner capability. The `MicroVmRuntimeProfile` records the engine, virtualization backend, runner capability/version, supported guest artifact profile, verified kernel/initrd/rootfs/disk/guest-image identities, declared launch bindings, lease, heartbeat, bounded logs, resources, and redacted output receipts before mounts, block devices, networks, vsock channels, metadata, capability handles, or outputs are attached. HermitOS-style unikernels are modeled as guest artifacts under this VM/microVM boundary, with guest image and loader/hypervisor artifacts verified separately. |

All host kinds share the same runtime-facing lifecycle: resolve artifact, start, stop, health, route/call handling, logs, receipts, and capability-scoped handles. Manifests, logs, and receipts must carry opaque handles, hashes, or redacted summaries rather than raw secrets, tickets, private keys, cluster cookies, or connection strings.

OCI artifact lowering is represented by `OciLoweringPlan` and `OciLoweringTarget` in `aspen-runtime-core`. The plan is portable data, not an OCI runtime: it records the original immutable `sha256:` image digest, entrypoint/args, selected isolated host boundary, derived rootfs/program/guest artifact identities, transformation provenance, declared handles, and unsupported-image diagnostics. Admission fails closed unless an `OciImage` declaration lowers into `MicroVm`, `Hyperlight`, `Wasm`, or a VM-backed `Unikernel` target; every derived artifact identity is content-addressed; every mount/env/network/host-call handle is declared; and lowering receipts redact runner identity, derived artifacts, capability handles, diagnostics, registry credentials, raw environment secrets, mutable tags, ambient host paths, tokens, and private material. `RuntimeHostKind::OciContainer` remains only as a dev/unsafe marker for future local smokes and is rejected as the default production boundary.

WASM runner profiles are represented by `WasmRuntimeProfile` in `aspen-runtime-core`. The profile is portable data, not a live Wasmtime/Wasmer process: it records a verified `WasmModule`, ABI version, entrypoint, node-local runner capability/version, deterministic extension or service-fragment mode, finite fuel/memory/time/input/output limits, declared host-function bindings, pure instantiate/call/stop/observe lifecycle state, and receipt-backed output summaries. Admission fails closed unless the unit uses `RuntimeHostKind::Wasm`, the module hash/ABI/entrypoint match, the runner advertises the ABI and limits, every host function is backed by a declared capability handle, deterministic extension mode avoids ambient network/clock/secret functions, service-fragment mode declares route ownership through runtime-service-core, and outputs/diagnostics are secret-safe. This keeps hooks, policies, plugins, and compatible service fragments bounded without turning every Aspen application into a WASM plugin.

Hyperlight runner profiles are represented by `HyperlightRuntimeProfile` in `aspen-runtime-core`. The profile is still portable data, not a live Hyperlight process: it records a verified `HyperlightImage`, the ABI/artifact profile accepted by the runner, node-local runner capability/version, finite resource policy, declared host-call bindings, and receipt-backed output summaries. Admission fails closed unless the unit uses `RuntimeHostKind::Hyperlight`, the image hash and entrypoint match, the runner advertises the requested ABI/artifact profile, every host call is backed by a declared capability handle, resources fit inside the runner policy, and outputs/diagnostics are secret-safe. This prevents Hyperlight-compatible workloads from gaining ambient files, sockets, devices, routes, environment, network, secrets, or undeclared host calls.

MicroVM runner profiles are represented by `MicroVmRuntimeProfile` in `aspen-runtime-core`. The profile is portable node-local runner data, not a booted VM: it records `MicroVmEngine` selection for Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm, or an equivalent engine; virtualization backend such as KVM; runner capability/version; supported guest artifact profile; verified `LinuxGuest` or `Unikernel` artifact identities; declared launch bindings; lease/heartbeat state; resource policy; bounded log summary; and redacted output artifact receipts. Admission fails closed unless the unit uses `RuntimeHostKind::MicroVm`, the selected runner engine matches, kernel/initrd/rootfs/disk/guest-image identities are verified, the runner supports the guest profile, the runtime launch capability is present, every mount/block/network/vsock/metadata/capability/output binding is declared and secret-safe, resources fit inside the runner policy, and logs/outputs/diagnostics are secret-safe. This prevents microVM workloads from gaining ambient host paths, devices, sockets, networks, secrets, or undeclared handles before boot.

HermitOS-style unikernels use the `HermitUnikernelArtifact` profile in `aspen-runtime-core`. The profile records the application image, `x86_64`/future target architecture, Hermit guest ABI, immutable image hash, launch profile (`Uhyve` or loader/QEMU), declared input channels, and redacted boot/serial evidence. Admission must prove a compatible microVM runner capability before launch: Uhyve profiles map to `MicroVmEngine::Uhyve`; loader/QEMU profiles map to `MicroVmEngine::QemuMicrovm` and must verify loader plus boot-profile hashes separately. Hermit remains a guest artifact under a VM/microVM host boundary, not an OCI container, native process, or generic ambient host execution path.

## Definitions

### Application

An application is the installable unit above the runtime. It declares one or more services, execution plans, routes, state bindings, capability requests, migrations, extension points, adapters, and receipt schemas.

```text
Application =
  manifest
  services
  execution plans
  routes
  state bindings
  capabilities
  migrations
  extension points
  adapters
  receipt schemas
```

Applications own product/domain semantics. Forge is an application because it owns repositories, refs, COBs, Git object semantics, sync, policy hooks, and user-facing route families.

### Service

A service is a long-lived supervised runtime unit. It may be built into `aspen-node`, launched from a Nix artifact, loaded as a WASM/Hyperlight guest, or run through a future VM/container adapter.

A service declares:

```text
ServiceSpec {
  name
  artifact
  desired_replicas
  placement
  resources
  capabilities
  routes
  state_bindings
  health_check
  restart_policy
  upgrade_policy
}
```

The runtime tracks concrete instances:

```text
ServiceInstance {
  service
  instance_id
  generation
  assigned_node
  status
  lease_epoch
  heartbeat_at
  health
  current_routes
}
```

### Executioner

The existing CI/job machinery should evolve into a generic execution service. `CI` becomes one producer of execution plans, not the name of the runtime primitive.

```text
ExecutionPlan = finite workflow template
ExecutionRun  = concrete durable run/attempt
ExecutionTask = runnable step inside a run
Runner        = node-local executor with capabilities/capacity
```

Examples that should use the executioner:

- Forge push checks.
- Nix builds.
- Scheduled repo mirror sync.
- Blob garbage collection.
- Deployment validation.
- User-submitted jobs.
- Hook handlers that should be finite and auditable.

### Plugin

A plugin is a bounded extension inside an application. It should not own the whole lifecycle or primary durable state of a first-class application.

Examples:

- Forge merge policy.
- Forge ref update validation.
- Docs renderer.
- Executioner lint rule.
- Package registry upload policy.

### Adapter

An adapter exposes an application through an external protocol or compatibility surface. Adapters are privileged edges, not the internal Aspen API.

Examples:

- Git smart protocol bridge.
- JJ bridge.
- HTTP/Nix binary cache gateway.
- SSH gateway.
- Federation bridge.

## New application lifecycle

A future app install should look like:

```bash
aspen app install forge
```

The runtime should perform these durable transitions:

```text
verify package/artifact/signature
  -> evaluate requested capabilities
  -> reserve app state prefixes/namespaces
  -> run or schedule migrations
  -> register service specs
  -> register routes
  -> register execution plans/schedules
  -> start desired service instances
  -> write install receipt
```

A package registry application might declare:

```text
AppManifest {
  name: "packages"
  version: "0.1.0"

  services: ["package-index", "package-download"]
  executions: ["verify-upload", "rebuild-search-index", "gc-orphans"]
  routes: ["packages.publish", "packages.resolve", "packages.download", "packages.search"]

  state:
    kv_prefixes: ["/apps/packages/"]
    blob_namespaces: ["packages"]

  capabilities:
    kv.readwrite("/apps/packages/")
    blob.readwrite("packages")
    execution.submit("verify-upload")
    route.expose("packages.*")
}
```

At runtime:

```text
user calls packages.publish
  -> route registry finds package service
  -> service validates capability and writes package metadata to KV/blob
  -> service submits verify-upload ExecutionRun
  -> executioner assigns run to a capable runner
  -> receipt records upload, verification, artifacts, and failures
```

## Distributed and temporal model

Runtime state should be represented as durable desired/current state transitions in Raft/KV. Node-local runners and service hosts reconcile against that state.

For finite execution:

```text
Submitted
  -> Pending
  -> Assigned(node, lease_epoch)
  -> Running(node, attempt)
  -> StopRequested | Succeeded | Failed
  -> RetryScheduled | Reassigned | Cancelled | Completed
  -> ReceiptWritten
```

For services:

```text
Desired(ServiceSpec)
  -> InstanceAssigned(node, generation, lease_epoch)
  -> Starting
  -> Healthy | Unhealthy
  -> Stopping | Failed
  -> Restarted | Reassigned | Drained
  -> ReceiptWritten
```

Moving work between nodes should normally mean stop/fail/lease-expire and restart another attempt elsewhere. Aspen should not require live process memory migration for the first runtime contract.

## Current Aspen anchors

These are the current repo seams that should be folded into the runtime model rather than ignored.

| Existing seam | Current anchor | Runtime interpretation |
| --- | --- | --- |
| Durable jobs model | `crates/aspen-jobs-core/src/lib.rs` (`JobSpec`, `Schedule`, `RetryPolicy`, `JobStatus`, job KV/heartbeat prefixes) | Foundation for `ExecutionPlan`, `ExecutionRun`, task routing, schedules, retries, heartbeats. |
| CI config/core logic | `crates/aspen-ci-core/src/lib.rs` (`PipelineConfig`, `StageConfig`, `JobConfig`, trigger/resource helpers) | CI becomes an application/profile that produces generic execution plans. |
| CI orchestrator/trigger | `crates/aspen-ci/src/orchestrator/`, `crates/aspen-ci/src/trigger/service.rs` | Fold into executioner producer/orchestrator services. |
| CI executors | `crates/aspen-ci-executor-*` | Become execution runners by capability: shell, VM, Nix. |
| Native job workers | `crates/aspen-jobs-worker-*` | Become execution runner adapters or runtime maintenance services. |
| Forge coordinator | `crates/aspen-forge/src/node.rs` (`ForgeNode::new`) | Wrap as a first-class `ForgeService` without changing internal domain logic first. |
| Handler registry | `crates/aspen-rpc-handlers/src/registry.rs` (`NativeHandlerPlan`, `HandlerRegistry`) | Move app handler registration behind service route registration. |
| Plugin manifests | `crates/aspen-plugin-api/src/manifest.rs` (`PluginManifest`) | Reference for app extension manifests, not a replacement for applications. |
| Deploy state | `crates/aspen-deploy/src/types.rs` (`DeploymentStatus`, `NodeDeployStatus`, `DeployArtifact`) | Reuse lifecycle/state-machine and artifact lessons for service upgrades. |
| Receipts | `docs/operator-receipts.md`, `crates/aspen-dogfood/src/receipt.rs`, CI receipt APIs | Generalize to runtime install/start/stop/failover/execution receipts. |
| Federation app registry | `docs/FEDERATION.md` app registry and `required_app()` notes | Reference for app identity and route discoverability, but runtime apps need stronger lifecycle/state contracts. |

## Dependency map to design explicitly

### Runtime core should depend on

- Portable ID/status/spec types.
- Capability request/binding types from auth/core crates.
- Artifact references such as Nix store path, blob hash, WASM hash, native built-in symbol.
- Resource limit descriptors using bounded Tiger Style constants.
- Receipt/event schemas.

It should not depend on:

- `aspen-node` setup code.
- Concrete redb storage internals.
- Concrete Iroh endpoint manager internals.
- Shell process execution.
- Nix build implementation.
- Forge domain logic.
- CI-specific pipeline semantics.

### Runtime service shell should depend on

- Runtime core types.
- KV/cluster trait APIs.
- Route registry and handler dispatch abstractions.
- Health/metrics/logging facades.
- Node-local service host implementations.

### Executioner should depend on

- Runtime core.
- Jobs-core model pieces or a successor extracted from it.
- Runner capability model.
- Artifact/log/receipt stores.
- Optional adapters for shell, Nix, VM, WASM, Hyperlight.

### Applications should depend on

- Runtime SDK/ABI.
- State/capability bindings granted to that application.
- Domain-specific protocol crates.

Applications should not depend directly on node startup wiring or bypass runtime route/capability registration.

## Likely crate split

Names are provisional, but the dependency direction should be clear:

```text
aspen-runtime-core
  pure specs: AppManifest, ServiceSpec, ExecutionPlan, ExecutionRun,
  RuntimeArtifact, RuntimeCapability, RuntimeReceipt, Placement, Resources

aspen-runtime-services
  service reconciler, service host trait, route binding, health, supervision

aspen-runtime-execution
  executioner scheduler, run state machine, runner registry, task assignment

aspen-runtime-sdk
  host ABI used by app services/guests: calls, timers, KV/blob handles,
  logs, metrics, health, cancellation, capability introspection

aspen-app-registry
  installed apps, versions, migrations, route ownership, extension points
```

Existing crates can migrate gradually:

```text
aspen-ci-core            -> execution plan/profile pieces
aspen-ci                -> CI application + executioner producer
aspen-ci-executor-*     -> execution runner adapters
aspen-jobs-core         -> execution run/task/schedule base or dependency
aspen-jobs              -> execution queue/scheduler shell, then executioner
aspen-forge             -> Forge domain service implementation
aspen-forge-handler     -> route adapter for ForgeService during migration
```

## Overlooked or high-risk seams

1. **Route ownership and conflicts**: app install must reject duplicate route families unless an explicit extension/priority rule exists.
2. **Capability escalation**: app manifests must request bounded KV/blob/execution/network scopes; plugins inherit from or are constrained by host app grants.
3. **Migrations**: schema changes need ordered, idempotent, receipt-producing migrations with rollback/diagnosis policy.
4. **Leases and fencing**: reassignment must prevent two nodes from completing the same exclusive execution or serving the same singleton route without an epoch check.
5. **Idempotency**: retries and failover require idempotency keys or output commit protocols.
6. **Durable execution history**: long-running workflows need an event/side-effect history, replay rules, and deterministic awaitable/timer APIs; simple status rows are not enough for Temporal/Flawless-style resume.
7. **Artifact provenance**: runtime artifacts need hash, signer, build provenance, and compatibility metadata.
8. **Secrets**: manifests and receipts must never contain raw tokens, tickets, private keys, cluster cookies, or connection strings; use handles and `[REDACTED]` in operator output.
9. **Resource admission**: placement must account for CPU, memory, disk, network, concurrency, and sandbox type before assignment.
10. **Observability cardinality**: app/service/run IDs must not create unbounded metric label sets.
11. **Federation boundary**: app identity may federate, but service placement and execution leases are cluster-local unless explicitly modeled cross-cluster.
12. **Compatibility edges**: HTTP/SSH/Git/Nix bridges should stay adapters; internal app calls should remain Aspen/Iroh/capability-routed.
13. **Native built-ins versus dynamic apps**: first migration should wrap built-ins as declared services before introducing dynamic install for every implementation type.

## Reference systems to borrow from

- Kubernetes controllers: desired state in specs, observed state, reconciliation loops, jobs that retry work to completion, deployments that roll out at controlled pace. Borrow reconciliation, not YAML sprawl or container-first assumptions.
- Erlang/OTP: applications as supervision trees; supervisors start, stop, and monitor workers. Borrow supervision semantics and failure containment, not BEAM language/runtime coupling.
- Lunatic (`https://github.com/lunatic-solutions/lunatic`, local reference `../lunatic`): WASM processes, fine-grained process permissions, supervision, channel message passing, and distributed nodes. Borrow isolation and per-process capability ideas for untrusted app units.
- Flawless (`https://flawless.dev/docs/`): Rust durable execution through WASM, deterministic replay, side-effect logs, idempotent external calls, and the ability to start a workflow on one machine and finish it on another. Borrow the side-effect/event-history model for long-running executioner workflows; do not assume every Aspen service must compile to Flawless-compatible WASM.
- Temporal (`https://docs.temporal.io/temporal`, `https://docs.temporal.io/workflow-execution`): durable Workflow Executions, Event History, Commands, Activities, Signals, timers, Worker Processes, retries, and Workflow Id/Run Id chains. Borrow the Workflow/Activity split, replay constraints, cancellation/status vocabulary, and worker/service separation; do not import Temporal's central service/database architecture wholesale because Aspen already has Raft/KV/Iroh authority.
- Pollen: content-addressed WASM seeds, `pln://seed/<name>/<fn>` / service-style calls, P2P artifact distribution, QUIC mesh, self-organising placement. Borrow service-call ergonomics and content-addressed distribution; reconcile with Aspen's Raft-backed authority instead of replacing it with fully coordinatorless placement.
- Nix/snix: reproducible artifact construction and content-addressed store semantics. Borrow artifact identity/provenance for runtime packages.
- Aspen dogfood/operator receipts: receipt-first operator evidence. Generalize from dogfood/CI receipts to runtime receipts.

## First implementation slice

The current `define-runtime-service-core` slice has implemented the contract/model and Forge wrapper portions of this plan. Treat the remaining bullets as migration track work, not as proof that the model crate is absent.

Completed in this slice:

1. Added `aspen-runtime-core` with pure types for `RuntimeServiceSpec`, `RuntimeServiceInstance`, `RuntimeArtifact`, `RuntimeCapabilityBinding`, `RuntimePlacementHints`, `RuntimeResources`, `RuntimeReceipt`, host kinds, policies, lifecycle, and health.
2. Added a static built-in Forge runtime-service manifest wrapper around existing Forge internals in `crates/aspen-forge/src/runtime_service.rs`.
3. Declared Forge routes through runtime route declarations for Git, repo/RPC, and health route families.
4. Added health/lifecycle receipt helpers that expose opaque capability handles and reject raw-secret diagnostics.

Remaining follow-up work:

1. Wire Forge route declarations into the active runtime route registry/reconciler when that shell exists.
2. Emit startup receipts from actual node/service lifecycle events instead of pure wrapper helpers.
3. Generalize CI/jobs into the executioner model after the service contract lands.
4. Add dynamic/non-built-in app formats only after the linked-native path is exercised.

## Open questions

- Should app manifests be Nickel-authored, Rust-derived, or both?
- What is the minimal route model: `ClientRpcRequest` variants, named `aspen://service/name/fn` calls, or both during migration?
- How much of `aspen-jobs-core` should be renamed versus kept as compatibility under executioner?
- What are the exact fencing semantics for singleton services and exclusive execution runs?
- Which receipt events are mandatory for app install/start/stop/upgrade/failover?
- What app capabilities are safe to delegate to plugins, and which require a privileged adapter/service?
- How should app migrations interact with Raft snapshots and rollback?
- What is the first dynamic non-built-in app format: Nix-built native binary, WASM component, Hyperlight guest, or plugin-only extension bundle?
