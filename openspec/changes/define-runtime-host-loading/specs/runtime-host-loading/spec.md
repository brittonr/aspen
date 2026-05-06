## ADDED Requirements

### Requirement: Runtime Host Taxonomy [r[runtime-host-loading.host-taxonomy]]

Aspen MUST classify runtime units by an explicit host kind before resolving or starting their executable artifact.

#### Scenario: Native built-in host selected [r[runtime-host-loading.host-taxonomy.native-built-in]]

- GIVEN a first-party Aspen service such as Forge, Executioner, snix/cache, or federation
- WHEN the service is enabled as a native runtime unit
- THEN its runtime declaration SHALL use a built-in native host kind
- AND the artifact identity SHALL be the built-in service name plus node/build version rather than a dynamically loaded native library path

#### Scenario: WASM host selected [r[runtime-host-loading.host-taxonomy.wasm]]

- GIVEN a runtime unit supplied as a WASM artifact
- WHEN the runtime resolves the unit
- THEN the declaration SHALL include the module hash, ABI version, entrypoint, resource limits, and capability bindings required for instantiation

#### Scenario: Hyperlight host selected [r[runtime-host-loading.host-taxonomy.hyperlight]]

- GIVEN an isolated execution task or service declares a Hyperlight artifact
- WHEN the scheduler assigns the unit
- THEN the assigned node SHALL have a compatible Hyperlight runner before the unit can transition to running

#### Scenario: OCI container host selected [r[runtime-host-loading.host-taxonomy.oci-container]]

- GIVEN a runtime unit declares an OCI image artifact
- WHEN the runtime selects an OCI/container host boundary
- THEN the declaration SHALL use an immutable image digest rather than a mutable tag for accepted execution
- AND it SHALL state the runner, entrypoint, arguments, mounts, environment handles, network policy, and capability bindings needed before start

#### Scenario: MicroVM host selected [r[runtime-host-loading.host-taxonomy.microvm]]

- GIVEN a runtime unit requires a Firecracker or Cloud Hypervisor boundary
- WHEN the scheduler assigns the unit
- THEN the assigned node SHALL advertise compatible KVM and microVM runner capability before the unit can transition to running

#### Scenario: Unikernel artifact selected [r[runtime-host-loading.host-taxonomy.unikernel]]

- GIVEN a runtime unit declares a HermitOS-style unikernel artifact
- WHEN the runtime resolves the unit
- THEN the declaration SHALL model the unikernel as a guest artifact that runs under a VM or microVM host boundary, not as an ordinary host process

#### Scenario: External native process host selected [r[runtime-host-loading.host-taxonomy.native-process]]

- GIVEN an operator-installed trusted native binary is allowed by a future runtime policy
- WHEN the runtime resolves the unit
- THEN the declaration SHALL model it as a separate native process artifact with a hash or store path and a host-ABI boundary

### Requirement: Native Built-In Service Loading [r[runtime-host-loading.native-built-in]]

Aspen MUST load first-party native services through a linked built-in service factory registry rather than an in-process native dynamic plugin system.

#### Scenario: Built-in service factory starts Forge [r[runtime-host-loading.native-built-in.forge-start]]

- GIVEN Raft-backed desired state enables the Forge service
- AND the node binary contains the Forge built-in service factory
- WHEN the runtime reconciler starts the service
- THEN it SHALL construct the native Forge service through the built-in registry
- AND it SHALL register Forge routes through the runtime route table
- AND it SHALL emit service-start and route-registration receipts without exposing raw credentials

#### Scenario: Native dynamic plugin rejected as default [r[runtime-host-loading.native-built-in.dynamic-plugin-rejected]]

- GIVEN a runtime extension requests in-process native dynamic loading
- WHEN the request is evaluated against the default host-loading policy
- THEN Aspen SHALL reject it unless a separate future OpenSpec explicitly accepts that unsafe ABI boundary
- AND the rejection rationale SHALL prefer linked built-ins, external native processes, WASM, or Hyperlight depending on trust and isolation needs

### Requirement: Content-Addressed Dynamic Artifact Loading [r[runtime-host-loading.dynamic-artifacts]]

Aspen MUST verify dynamic runtime artifacts by content identity before instantiating WASM modules, starting Hyperlight units, launching OCI/microVM guests, or launching external native processes.

#### Scenario: WASM artifact verified before instantiation [r[runtime-host-loading.dynamic-artifacts.wasm-verified]]

- GIVEN a WASM runtime unit declares a module hash and ABI version
- WHEN a node prepares to instantiate the module
- THEN the node SHALL fetch the module from an Aspen-approved artifact store
- AND it SHALL verify the content identity before exposing any host functions
- AND it SHALL fail closed if the hash, signature, ABI, fuel, memory, or timeout policy is invalid

#### Scenario: Hyperlight artifact verified before start [r[runtime-host-loading.dynamic-artifacts.hyperlight-verified]]

- GIVEN a Hyperlight execution run is assigned to a node
- WHEN the node prepares the guest image or program
- THEN the Hyperlight runner SHALL verify the artifact identity before starting the isolated unit
- AND outputs SHALL be recorded as Aspen blob/snix artifacts or explicit receipt fields

#### Scenario: OCI image verified before container start [r[runtime-host-loading.dynamic-artifacts.oci-verified]]

- GIVEN an OCI-backed runtime unit declares an image digest
- WHEN the node prepares the root filesystem or container bundle
- THEN the runner SHALL resolve and verify the immutable image digest and signature/provenance policy before process creation
- AND mutable tags SHALL NOT be accepted as the durable execution identity
- AND layer/rootfs materialization SHALL be recorded as bounded receipt data without raw credentials

#### Scenario: MicroVM guest artifact verified before boot [r[runtime-host-loading.dynamic-artifacts.microvm-verified]]

- GIVEN a microVM-backed runtime unit declares kernel, initrd, rootfs, or unikernel artifacts
- WHEN the node prepares the guest
- THEN the runner SHALL verify every declared guest artifact identity before boot
- AND guest inputs SHALL be sealed through declared capability handles rather than ambient host paths

#### Scenario: External native process verified before spawn [r[runtime-host-loading.dynamic-artifacts.native-process-verified]]

- GIVEN a future policy allows an external native process runtime unit
- WHEN the runtime prepares to spawn it
- THEN the runtime SHALL verify the binary artifact identity before process creation
- AND it SHALL attach only declared IPC, filesystem, environment, and capability handles

### Requirement: Host-Independent Runtime Lifecycle [r[runtime-host-loading.lifecycle]]

Aspen MUST expose a host-independent lifecycle model for runtime units regardless of whether the implementation is native, WASM, Hyperlight, OCI/container, microVM/unikernel, or an external native process.

#### Scenario: Common lifecycle fields [r[runtime-host-loading.lifecycle.common-fields]]

- GIVEN any runtime unit declaration
- WHEN it is persisted or reconciled
- THEN it SHALL include enough structured data to derive artifact identity, host kind, entrypoint or built-in name, capabilities, resources, placement constraints, route ownership, health policy, logs, and receipt policy

#### Scenario: Common lifecycle transitions [r[runtime-host-loading.lifecycle.common-transitions]]

- GIVEN any runtime unit instance
- WHEN the runtime starts, stops, restarts, health-checks, upgrades, or fails the unit
- THEN the state machine SHALL record host-independent lifecycle transitions and receipts
- AND host-specific details SHALL remain attached as bounded diagnostic fields or artifacts

### Requirement: Capability-Scoped Host Bindings [r[runtime-host-loading.capability-bindings]]

Aspen MUST bind runtime unit authority through explicit capability-scoped handles instead of ambient access to cluster state, network, credentials, or local filesystem paths.

#### Scenario: Native built-in receives typed handles [r[runtime-host-loading.capability-bindings.native-handles]]

- GIVEN a native built-in service starts under the runtime
- WHEN the runtime constructs its service context
- THEN the context SHALL contain only the typed handles required by the service manifest and node policy
- AND service receipts SHALL describe granted authority at a non-secret level

#### Scenario: WASM receives host functions only [r[runtime-host-loading.capability-bindings.wasm-host-functions]]

- GIVEN a WASM module is instantiated
- WHEN it calls Aspen host functions
- THEN each host function SHALL check the module's bound capability handles before accessing KV, blobs, routes, execution, secrets, or logs

#### Scenario: Hyperlight receives narrow host ABI [r[runtime-host-loading.capability-bindings.hyperlight-host-abi]]

- GIVEN a Hyperlight unit starts
- WHEN the guest requests Aspen operations
- THEN the runner SHALL expose only declared devices, sockets, files, environment variables, and host calls
- AND denied operations SHALL fail closed with bounded diagnostics

#### Scenario: OCI container receives declared runtime handles [r[runtime-host-loading.capability-bindings.oci-handles]]

- GIVEN an OCI/container unit starts
- WHEN the runner creates its bundle or process
- THEN the runner SHALL expose only declared mounts, environment handles, network policy, user namespace, filesystem permissions, and host calls
- AND the runtime SHALL treat ordinary container isolation as weaker than Hyperlight or microVM isolation for hostile workloads

#### Scenario: MicroVM guest receives sealed inputs [r[runtime-host-loading.capability-bindings.microvm-sealed-inputs]]

- GIVEN a microVM or unikernel guest starts
- WHEN the runner injects inputs, configuration, or credentials
- THEN the runner SHALL use sealed files, devices, sockets, or host calls declared by the capability binding
- AND it SHALL NOT pass raw secrets through kernel arguments, serial logs, mutable image layers, or receipts

#### Scenario: Secrets never serialized into manifests or receipts [r[runtime-host-loading.capability-bindings.secret-redaction]]

- GIVEN a runtime manifest, capability binding, log, or receipt is persisted or displayed
- WHEN it references credentials, tickets, private keys, cluster cookies, connection strings, or secret material
- THEN the persisted/displayed data SHALL contain only opaque handles, hashes, redacted summaries, or paths to protected owner-only files
- AND it SHALL NOT contain raw secret values

### Requirement: UCAN-Shaped Runtime Delegation [r[runtime-host-loading.ucan-delegation]]

Aspen MUST evaluate UCAN-style ability/resource/proof/caveat vocabulary as the preferred model for runtime capability delegation before inventing an incompatible authority language.

#### Scenario: UCAN reference reviewed [r[runtime-host-loading.ucan-delegation.reference-reviewed]]

- GIVEN the sibling `../ucan/` repository is available
- WHEN runtime capability binding implementation begins
- THEN the implementation plan SHALL inspect the UCAN core/shell boundary and reuse or adapt ability, resource, proof-chain, caveat, and structured-denial concepts where they fit Aspen runtime units

#### Scenario: UCAN dependency boundary preserved [r[runtime-host-loading.ucan-delegation.boundary-preserved]]

- GIVEN Aspen runtime-core needs portable capability binding types
- WHEN UCAN concepts are adopted
- THEN the portable runtime crate SHALL avoid depending on UCAN std-shell behavior unless a separate dependency-boundary review proves it is acceptable
- AND cryptographic verification, resolver I/O, and backend traversal SHALL remain explicit runtime/shell boundaries

### Requirement: Verified Admission Predicates [r[runtime-host-loading.verified-admission]]

Aspen MUST use verified-logic-backed predicates for finite runtime admission checks when the predicate is narrow, structural, and already modeled or practical to model in `../verified-logic/`.

#### Scenario: Verified-logic reference reviewed [r[runtime-host-loading.verified-admission.reference-reviewed]]

- GIVEN the sibling `../verified-logic/` repository is available
- WHEN runtime host-loading implementation begins
- THEN the implementation plan SHALL inspect existing UCAN, artifact, bounded-resource, and durable-execution primitives before adding local unverified equivalents

#### Scenario: Structural admission selected [r[runtime-host-loading.verified-admission.structural-selected]]

- GIVEN a runtime admission rule checks finite structure such as host-kind validity, artifact hash byte shape, resource bound shape, ability/resource syntax, proof-hop depth, or typed caveat payload shape
- WHEN a verified predicate exists or can be added in a focused slice
- THEN Aspen SHALL route the runtime-core check through that predicate or document why the seam is not yet suitable for verification

#### Scenario: Verification boundary not overclaimed [r[runtime-host-loading.verified-admission.boundary-not-overclaimed]]

- GIVEN a runtime check depends on cryptographic strength, sandbox implementation, scheduler fairness, network resolution, filesystem I/O, or external policy backends
- WHEN verification evidence is reported
- THEN Aspen SHALL identify those surfaces as trusted runtime boundaries unless a separate proof explicitly covers them
