# runtime-host-loading Specification

## Purpose

This specification defines Aspen's runtime host-loading taxonomy and shared lifecycle, artifact-verification, capability-binding, UCAN-delegation, and verified-admission requirements for native built-ins, external native processes, WASM, Hyperlight, OCI/container, microVM, and unikernel runtime units.
## Requirements
### Requirement: Runtime Host Taxonomy
Aspen MUST classify runtime units by an explicit host kind before resolving or starting their executable artifact.
ID: r[runtime-host-loading.host-taxonomy]

#### Scenario: Native built-in host selected
First-party service declarations MUST use a built-in native host kind rather than dynamic native libraries.
ID: r[runtime-host-loading.host-taxonomy.native-built-in]
- GIVEN a first-party Aspen service such as Forge, Executioner, snix/cache, or federation
- WHEN the service is enabled as a native runtime unit
- THEN its runtime declaration SHALL use a built-in native host kind
- AND the artifact identity SHALL be the built-in service name plus node/build version rather than a dynamically loaded native library path

#### Scenario: WASM host selected
WASM runtime declarations MUST include content identity, ABI, entrypoint, limits, and capabilities.
ID: r[runtime-host-loading.host-taxonomy.wasm]
- GIVEN a runtime unit supplied as a WASM artifact
- WHEN the runtime resolves the unit
- THEN the declaration SHALL include the module hash, ABI version, entrypoint, resource limits, and capability bindings required for instantiation

#### Scenario: Hyperlight host selected
Hyperlight runtime assignments MUST require compatible node runner capability.
ID: r[runtime-host-loading.host-taxonomy.hyperlight]
- GIVEN an isolated execution task or service declares a Hyperlight artifact
- WHEN the scheduler assigns the unit
- THEN the assigned node SHALL have a compatible Hyperlight runner before the unit can transition to running

#### Scenario: OCI container host selected
OCI runtime declarations MUST use immutable image identity and bounded host handles.
ID: r[runtime-host-loading.host-taxonomy.oci-container]
- GIVEN a runtime unit declares an OCI image artifact
- WHEN the runtime selects an OCI/container host boundary
- THEN the declaration SHALL use an immutable image digest rather than a mutable tag for accepted execution
- AND it SHALL state the runner, entrypoint, arguments, mounts, environment handles, network policy, and capability bindings needed before start

#### Scenario: MicroVM host selected
MicroVM runtime assignments MUST require compatible virtualization and runner capability.
ID: r[runtime-host-loading.host-taxonomy.microvm]
- GIVEN a runtime unit requires a Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm, or equivalent VM boundary
- WHEN the scheduler assigns the unit
- THEN the assigned node SHALL advertise compatible virtualization and runner capability before the unit can transition to running

#### Scenario: Unikernel artifact selected
Unikernel artifacts MUST be modeled as guest artifacts under a VM or microVM boundary.
ID: r[runtime-host-loading.host-taxonomy.unikernel]
- GIVEN a runtime unit declares a HermitOS-style unikernel artifact
- WHEN the runtime resolves the unit
- THEN the declaration SHALL model the unikernel as a guest artifact that runs under a VM or microVM host boundary, not as an ordinary host process
- AND it SHALL separately identify the Hermit application image and any loader, hypervisor, or boot profile artifacts required to launch it

#### Scenario: External native process host selected
External native process units MUST use a separate process artifact and host-ABI boundary.
ID: r[runtime-host-loading.host-taxonomy.native-process]
- GIVEN an operator-installed trusted native binary is allowed by a future runtime policy
- WHEN the runtime resolves the unit
- THEN the declaration SHALL model it as a separate native process artifact with a hash or store path and a host-ABI boundary

### Requirement: Native Built-In Service Loading
Aspen MUST load first-party native services through a linked built-in service factory registry rather than an in-process native dynamic plugin system.
ID: r[runtime-host-loading.native-built-in]

#### Scenario: Built-in service factory starts Forge
The native built-in factory scenario MUST model Forge startup through a built-in registry and runtime route declaration.
ID: r[runtime-host-loading.native-built-in.forge-start]
- GIVEN Raft-backed desired state enables the Forge service
- AND the node binary contains the Forge built-in service factory
- WHEN the runtime reconciler starts the service
- THEN it SHALL construct the native Forge service through the built-in registry
- AND it SHALL register Forge routes through the runtime route table
- AND it SHALL emit service-start and route-registration receipts without exposing raw credentials

#### Scenario: Native dynamic plugin rejected as default
The default host-loading policy MUST reject in-process native dynamic plugins unless a future spec accepts the unsafe ABI boundary.
ID: r[runtime-host-loading.native-built-in.dynamic-plugin-rejected]
- GIVEN a runtime extension requests in-process native dynamic loading
- WHEN the request is evaluated against the default host-loading policy
- THEN Aspen SHALL reject it unless a separate future OpenSpec explicitly accepts that unsafe ABI boundary
- AND the rejection rationale SHALL prefer linked built-ins, external native processes, WASM, or Hyperlight depending on trust and isolation needs

### Requirement: Content-Addressed Dynamic Artifact Loading
Aspen MUST verify dynamic runtime artifacts by content identity before instantiating WASM modules, starting Hyperlight units, launching OCI/microVM guests, or launching external native processes.
ID: r[runtime-host-loading.dynamic-artifacts]

#### Scenario: WASM artifact verified before instantiation
WASM artifact admission MUST fail closed before host functions are exposed when identity, ABI, or resource policy is invalid.
ID: r[runtime-host-loading.dynamic-artifacts.wasm-verified]
- GIVEN a WASM runtime unit declares a module hash and ABI version
- WHEN a node prepares to instantiate the module
- THEN the node SHALL fetch the module from an Aspen-approved artifact store
- AND it SHALL verify the content identity before exposing any host functions
- AND it SHALL fail closed if the hash, signature, ABI, fuel, memory, or timeout policy is invalid

#### Scenario: Hyperlight artifact verified before start
Hyperlight artifact admission MUST verify guest identity before start and record outputs as artifacts or receipt fields.
ID: r[runtime-host-loading.dynamic-artifacts.hyperlight-verified]
- GIVEN a Hyperlight execution run is assigned to a node
- WHEN the node prepares the guest image or program
- THEN the Hyperlight runner SHALL verify the artifact identity before starting the isolated unit
- AND outputs SHALL be recorded as Aspen blob/snix artifacts or explicit receipt fields

#### Scenario: Hermit guest artifact verified before start
Hermit unikernel guest admission MUST distinguish guest application image, loader, hypervisor, and boot-profile artifacts before start.
ID: r[runtime-host-loading.dynamic-artifacts.hermit-guest-verified]
- GIVEN a HermitOS-style unikernel runtime unit is assigned to a microVM-capable node
- WHEN the node prepares the guest application image and loader or hypervisor profile
- THEN the runner SHALL verify the guest artifact identity before start
- AND the receipt SHALL record selected engine and artifact hashes rather than mutable host paths, raw kernel args, or environment secrets

#### Scenario: External native process verified before spawn
External native process admission MUST verify binary identity before process creation and attach only declared handles.
ID: r[runtime-host-loading.dynamic-artifacts.native-process-verified]
- GIVEN a future policy allows an external native process runtime unit
- WHEN the runtime prepares to spawn it
- THEN the runtime SHALL verify the binary artifact identity before process creation
- AND it SHALL attach only declared IPC, filesystem, environment, and capability handles

### Requirement: Host-Independent Runtime Lifecycle
Aspen MUST expose a host-independent lifecycle model for runtime units regardless of whether the implementation is native, WASM, Hyperlight, OCI/container, microVM, unikernel, or an external native process.
ID: r[runtime-host-loading.lifecycle]

#### Scenario: Common lifecycle fields
Runtime unit declarations MUST include enough structured fields for host-independent reconciliation and receipts.
ID: r[runtime-host-loading.lifecycle.common-fields]
- GIVEN any runtime unit declaration
- WHEN it is persisted or reconciled
- THEN it SHALL include enough structured data to derive artifact identity, host kind, entrypoint or built-in name, capabilities, resources, placement constraints, route ownership, health policy, logs, and receipt policy

#### Scenario: Common lifecycle transitions
Runtime unit instances MUST record host-independent lifecycle transitions and bounded host-specific diagnostics.
ID: r[runtime-host-loading.lifecycle.common-transitions]
- GIVEN any runtime unit instance
- WHEN the runtime starts, stops, restarts, health-checks, upgrades, or fails the unit
- THEN the state machine SHALL record host-independent lifecycle transitions and receipts
- AND host-specific details SHALL remain attached as bounded diagnostic fields or artifacts

### Requirement: Capability-Scoped Host Bindings
Aspen MUST bind runtime unit authority through explicit capability-scoped handles instead of ambient access to cluster state, network, credentials, or local filesystem paths.
ID: r[runtime-host-loading.capability-bindings]

#### Scenario: Native built-in receives typed handles
Native built-in service contexts MUST contain only typed handles required by the manifest and policy.
ID: r[runtime-host-loading.capability-bindings.native-handles]
- GIVEN a native built-in service starts under the runtime
- WHEN the runtime constructs its service context
- THEN the context SHALL contain only the typed handles required by the service manifest and node policy
- AND service receipts SHALL describe granted authority at a non-secret level

#### Scenario: WASM receives host functions only
WASM host functions MUST check bound capability handles before accessing Aspen subsystems.
ID: r[runtime-host-loading.capability-bindings.wasm-host-functions]
- GIVEN a WASM module is instantiated
- WHEN it calls Aspen host functions
- THEN each host function SHALL check the module's bound capability handles before accessing KV, blobs, routes, execution, secrets, or logs

#### Scenario: Hyperlight receives narrow host ABI
Hyperlight guests MUST receive only declared devices, sockets, files, environment variables, and host calls.
ID: r[runtime-host-loading.capability-bindings.hyperlight-host-abi]
- GIVEN a Hyperlight unit starts
- WHEN the guest requests Aspen operations
- THEN the runner SHALL expose only declared devices, sockets, files, environment variables, and host calls
- AND denied operations SHALL fail closed with bounded diagnostics

#### Scenario: Secrets never serialized into manifests or receipts
Runtime manifests, logs, and receipts MUST never persist or display raw secret values.
ID: r[runtime-host-loading.capability-bindings.secret-redaction]
- GIVEN a runtime manifest, capability binding, log, or receipt is persisted or displayed
- WHEN it references credentials, tickets, private keys, cluster cookies, connection strings, or secret material
- THEN the persisted/displayed data SHALL contain only opaque handles, hashes, redacted summaries, or paths to protected owner-only files
- AND it SHALL NOT contain raw secret values

### Requirement: UCAN-Shaped Runtime Delegation
Aspen MUST evaluate UCAN-style ability/resource/proof/caveat vocabulary as the preferred model for runtime capability delegation before inventing an incompatible authority language.
ID: r[runtime-host-loading.ucan-delegation]

#### Scenario: UCAN reference reviewed
Runtime capability binding implementation MUST inspect UCAN core/shell boundaries and reusable delegation vocabulary.
ID: r[runtime-host-loading.ucan-delegation.reference-reviewed]
- GIVEN the sibling `../ucan/` repository is available
- WHEN runtime capability binding implementation begins
- THEN the implementation plan SHALL inspect the UCAN core/shell boundary and reuse or adapt ability, resource, proof-chain, caveat, and structured-denial concepts where they fit Aspen runtime units

#### Scenario: UCAN dependency boundary preserved
Portable runtime capability types MUST avoid depending on UCAN std-shell behavior unless separately justified.
ID: r[runtime-host-loading.ucan-delegation.boundary-preserved]
- GIVEN Aspen runtime-core needs portable capability binding types
- WHEN UCAN concepts are adopted
- THEN the portable runtime crate SHALL avoid depending on UCAN std-shell behavior unless a separate dependency-boundary review proves it is acceptable
- AND cryptographic verification, resolver I/O, and backend traversal SHALL remain explicit runtime/shell boundaries

### Requirement: Verified Admission Predicates
Aspen MUST use verified-logic-backed predicates for finite runtime admission checks when the predicate is narrow, structural, and already modeled or practical to model in `../verified-logic/`.
ID: r[runtime-host-loading.verified-admission]

#### Scenario: Verified-logic reference reviewed
Runtime host-loading implementation MUST inspect existing verified-logic primitives before adding local unverified equivalents.
ID: r[runtime-host-loading.verified-admission.reference-reviewed]
- GIVEN the sibling `../verified-logic/` repository is available
- WHEN runtime host-loading implementation begins
- THEN the implementation plan SHALL inspect existing UCAN, artifact, bounded-resource, and durable-execution primitives before adding local unverified equivalents

#### Scenario: Structural admission selected
Runtime admission SHOULD route narrow structural checks through verified predicates when an appropriate proof exists; otherwise it MUST document why the seam is not yet formally verified.
ID: r[runtime-host-loading.verified-admission.structural-selected]
- GIVEN a runtime admission rule checks finite structure such as host-kind validity, artifact hash byte shape, resource bound shape, ability/resource syntax, proof-hop depth, or typed caveat payload shape
- WHEN a verified predicate exists or can be added in a focused slice
- THEN Aspen SHALL route the runtime-core check through that predicate or document why the seam is not yet suitable for verification

#### Scenario: Verification boundary not overclaimed
Runtime verification evidence MUST identify cryptographic, sandbox, scheduler, network, filesystem, and external policy surfaces as trusted boundaries unless separately proven.
ID: r[runtime-host-loading.verified-admission.boundary-not-overclaimed]
- GIVEN a runtime check depends on cryptographic strength, sandbox implementation, scheduler fairness, network resolution, filesystem I/O, or external policy backends
- WHEN verification evidence is reported
- THEN Aspen SHALL identify those surfaces as trusted runtime boundaries unless a separate proof explicitly covers them

### Requirement: Hermit Unikernel Runtime Profile [r[runtime-host-loading.hermit-profile]]
Aspen MUST model HermitOS-style unikernels as verified guest artifacts that run under a VM or microVM host boundary rather than as OCI containers or native host processes.

#### Scenario: Hermit artifact identity is explicit [r[runtime-host-loading.hermit-profile.artifact-identity]]
- GIVEN a runtime unit declares a HermitOS-style unikernel
- WHEN the runtime resolves the artifact
- THEN it SHALL identify the Hermit application image, target architecture, guest ABI/profile, and content hash separately from the host runner

#### Scenario: Uhyve launch profile is capability-gated [r[runtime-host-loading.hermit-profile.uhyve]]
- GIVEN a Hermit guest is assigned to a Uhyve launch profile
- WHEN node admission evaluates the assignment
- THEN admission SHALL require a compatible Uhyve runner capability and SHALL verify the guest image before launch

#### Scenario: Loader or QEMU path verifies loader artifacts [r[runtime-host-loading.hermit-profile.loader-qemu]]
- GIVEN a Hermit guest uses a loader, QEMU microvm, or equivalent boot path
- WHEN the runner prepares the launch
- THEN it SHALL verify loader, boot profile, and guest image identities separately and SHALL record those identities in the receipt

#### Scenario: Hermit boot inputs do not carry secrets [r[runtime-host-loading.hermit-profile.secret-boundary]]
- GIVEN a Hermit guest requires configuration, capability handles, or runtime inputs
- WHEN the launch profile creates boot arguments, environment-like metadata, serial output, or receipts
- THEN it SHALL NOT include raw tokens, tickets, private keys, cluster cookies, connection strings, or other secret material

#### Scenario: Hermit input channels are explicit [r[runtime-host-loading.hermit-profile.input-channels]]
- GIVEN a Hermit guest requires configuration or capability handles
- WHEN the profile maps inputs into boot arguments, loader metadata, virtio/vsock channels, or host ABI shims
- THEN every input channel SHALL be declared and authorized before launch
- AND undeclared host filesystem, network, secret, device, and ambient access SHALL be denied by default

#### Scenario: Hermit serial output is bounded and redacted [r[runtime-host-loading.hermit-profile.serial-logs]]
- GIVEN a Hermit guest writes serial or console output
- WHEN the runner captures logs
- THEN it SHALL bound log size, redact known secret-bearing fields, and persist logs as Aspen-approved artifacts or redacted receipt fields
