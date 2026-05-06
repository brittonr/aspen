## Context

Aspen is moving toward a distributed infrastructure runtime: services are long-lived supervised units, jobs/execution runs are finite scheduled units, workflows add durable event/side-effect history, and applications package services, executions, routes, state, capabilities, migrations, and receipts.

The open architectural question is not whether all code becomes WASM. Aspen should support several host kinds behind one runtime contract:

- native built-ins for trusted first-party services,
- optional external native processes for trusted operator-installed binaries,
- WASM for bounded extension logic and portable untrusted modules,
- Hyperlight for isolated native-ish workloads and executioner jobs,
- OCI images/containers for compatibility with existing Linux workload packaging,
- microVMs and unikernel guests as later stronger-isolation host/artifact profiles.

Existing code already has pieces of this model: Forge is compiled into the node behind features and wired during startup; WASM plugin manifests exist; jobs/CI/executors provide finite execution concepts; Hyperlight support exists in plugin/job-adjacent areas; receipts exist in dogfood/deploy/operator flows. This change defines the target loading contract before implementation begins.

## Goals / Non-Goals

**Goals:**

- Define host kinds and loading behavior for native, WASM, Hyperlight, OCI/container, and microVM/unikernel units.
- Preserve native built-ins as the first implementation path for Forge and other first-party services.
- Make route registration, lifecycle, health, capabilities, logs, and receipts host-independent.
- Specify where `../verified-logic/` should be used for finite admission predicates.
- Specify how `../ucan/` should inform capability-token/delegation bindings without forcing an immediate dependency.

**Non-Goals:**

- No full runtime implementation in this change.
- No conversion of Forge into WASM.
- No dynamic native `dlopen` plugin system as the default architecture.
- No live process-memory migration between Aspen nodes.
- No requirement that every runtime service compile to WASM or Hyperlight.

## Decisions

### 1. Native first-party services are linked built-ins

**Choice:** First-party native services SHALL be normal Rust crates linked into `aspen-node` and registered through a built-in service factory registry.

**Rationale:** This matches current Forge reality, preserves type safety and direct Rust integration, avoids unstable native plugin ABI problems, and gives Aspen the runtime lifecycle/receipt model without a rewrite.

**Rejected alternative:** In-process native dynamic plugins as the main native extension model. They are rejected because they create ABI/version skew, symbol, supply-chain, crash-isolation, and capability-enforcement problems while providing little benefit over linked built-ins or separate processes.

**Implementation sketch:**

```rust
trait NativeServiceFactory {
    fn service_name(&self) -> &'static str;
    fn manifest(&self) -> ServiceManifest;
    fn create(&self, ctx: ServiceContext) -> anyhow::Result<Box<dyn AspenService>>;
}
```

A built-in artifact is addressed as `RuntimeArtifact::BuiltIn { name, version }`. The runtime reconciler looks up the factory by name and starts it when Raft-backed desired state requires the service.

### 2. External native binaries are separate processes, not native plugins

**Choice:** If Aspen supports independently deployable native code, it SHALL be modeled as a verified native process artifact launched out-of-process with a local IPC/host-ABI boundary.

**Rationale:** Separate processes provide a clearer crash boundary and make sandboxing, resource accounting, restart, and upgrade behavior observable.

**Implementation sketch:**

```text
RuntimeArtifact::NativeBinary { hash, store_path, entrypoint }
  -> verify artifact
  -> spawn process under restricted environment
  -> attach local socket/host ABI
  -> proxy route calls
  -> monitor health/exit
```

### 3. WASM loads by content-addressed module hash

**Choice:** WASM units SHALL be loaded from Aspen blob/snix artifacts by hash/signature, instantiated with bounded fuel/memory/time, and exposed only capability-scoped host functions.

**Rationale:** WASM is the right default for untrusted extension logic such as hooks, policies, small adapters, and deterministic activities. It should not be the mandatory form of every core service.

**Implementation sketch:**

```text
RuntimeArtifact::Wasm { module_hash, abi, entrypoint }
  -> fetch bytes
  -> verify hash/signature
  -> instantiate runtime
  -> bind allowed host calls
  -> call entrypoint
  -> record logs/receipts
```

### 4. Hyperlight loads by verified image/program artifact

**Choice:** Hyperlight units SHALL be assigned to nodes with a Hyperlight runner, started from verified artifacts, and connected to Aspen through a narrow host ABI/proxy.

**Rationale:** Hyperlight is best for isolated executioner jobs, builds, tests, risky adapters, tenant workloads, and possibly later long-lived services that need stronger isolation than native or WASM.

**Implementation sketch:**

```text
ExecutionRun Pending
  -> Assigned(node, host=Hyperlight, lease)
  -> runner fetches verified artifact
  -> runner starts isolated guest
  -> runtime streams logs/heartbeats
  -> outputs become blob/snix artifacts
  -> receipt records result/provenance
```

### 5. OCI is a packaging/compatibility profile, not sufficient isolation by itself

**Choice:** Aspen SHALL support OCI images as a runtime artifact profile and MAY run them through an OCI/container host kind when node policy allows it, but OCI image identity SHALL NOT by itself imply a strong sandbox boundary.

**Rationale:** OCI is the dominant packaging format for existing Linux workloads and is useful for adapters, CI jobs, language runtimes, and migration paths. However, ordinary containers share the host kernel and should be treated as weaker isolation than Hyperlight or microVMs unless paired with stronger runtimes such as Kata/gVisor-style boundaries in a later spec.

**Implementation sketch:**

```text
RuntimeArtifact::OciImage { image_digest, entrypoint, args }
  -> resolve by digest, not mutable tag
  -> verify signature/provenance policy
  -> materialize rootfs/layers through an approved store
  -> run under declared host: OciContainer | MicroVm-backed container | external runner
  -> attach only declared mounts/env/network/capability handles
  -> record image digest, runner, outputs, and receipt
```

### 6. MicroVMs and unikernels are stronger-isolation profiles

**Choice:** Firecracker and Cloud Hypervisor SHALL be modeled as microVM host engines when adopted. HermitOS-style unikernels SHALL be modeled as guest artifact profiles that run under a VM/microVM host boundary.

**Rationale:** MicroVMs are appropriate for high-isolation jobs, builds, tests, tenant workloads, and risky adapters. Unikernels are useful for small sealed Rust-native guests but are not a general Linux compatibility layer.

**Implementation sketch:**

```text
RuntimeHostKind::MicroVm { engine: Firecracker | CloudHypervisor }
RuntimeArtifact::LinuxGuest { kernel_hash, initrd_hash, rootfs_hash }
RuntimeArtifact::Unikernel { kind: HermitOS, image_hash }
```

### 7. Runtime-facing lifecycle is host-independent

**Choice:** Every host kind SHALL expose the same runtime-facing lifecycle surface: resolve artifact, start, stop, health, route/call handling, logs, receipts, and capability-scoped handles.

**Rationale:** This prevents parallel schedulers for each host type and lets Forge, Executioner, WASM hooks, Hyperlight jobs, OCI workloads, and microVM guests use one desired-state/reconciliation model.

### 8. Capabilities use UCAN-shaped delegation and verified admission where feasible

**Choice:** Runtime capability bindings SHALL be explicit data structures that can be checked before load/start and audited in receipts. The design SHALL use `../ucan/` as the reference for UCAN-style ability/resource/proof/caveat vocabulary, and use `../verified-logic/` for finite structural predicates when they are narrow enough to prove.

**Rationale:** Runtime units must not receive ambient Aspen authority. UCAN gives a mature delegation vocabulary; verified-logic can close finite admission seams such as host-kind validity, artifact hash shape, resource bound shape, ability/resource shape, and proof-hop/caveat structural constraints.

**Boundary:** Cryptographic signature verification, external storage, network resolution, and policy backends remain shell/runtime trust boundaries unless separately proven.

## Risks / Trade-offs

**Native built-ins have weak isolation** → Mitigate by using them only for trusted first-party services and preferring WASM/Hyperlight/microVM/native-process for dynamic or tenant-supplied code.

**OCI can be mistaken for a security boundary** → Mitigate by treating OCI primarily as a packaging format; require node policy to choose the actual host boundary, and prefer Hyperlight/microVMs for hostile workloads.

**Too much abstraction before code** → Mitigate with a small first implementation slice: define portable runtime-core types and wrap Forge as `BuiltIn("forge")` without changing Forge internals.

**UCAN overcoupling** → Mitigate by referencing UCAN vocabulary and bridge predicates first; avoid making Aspen runtime-core depend on the full UCAN shell until the minimal token/capability boundary is proven.

**Verified-logic overreach** → Mitigate by proving finite structural admission only; do not claim verification of cryptography, networking, scheduling fairness, or sandbox implementation.

**Hyperlight service complexity** → Mitigate by implementing Hyperlight first for finite Executioner runs, then considering long-lived Hyperlight services only after runner receipts/health/lease behavior are proven.

## Validation Plan

- Strict OpenSpec validation for this change.
- Future implementation tests for `RuntimeHostKind` serialization, artifact admission, route ownership, capability binding redaction, and host-kind-specific loading plans.
- Future docs/source-anchor tests keeping the runtime host contract discoverable from `docs/runtime-applications.md`.
- Future verified-logic evidence for finite admission predicates when introduced.
- Future UCAN bridge evidence showing runtime capability bindings can be expressed without leaking raw secrets into manifests or receipts.
