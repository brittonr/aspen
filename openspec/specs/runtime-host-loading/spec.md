# runtime-host-loading Specification

## Purpose

This specification defines Aspen's runtime host-loading taxonomy and shared lifecycle, artifact-verification, capability-binding, UCAN-delegation, and verified-admission requirements for native built-ins, external native processes, WASM, Hyperlight, OCI artifact lowering, microVM, and unikernel runtime units.
## Requirements
### Requirement: Runtime Host Taxonomy [r[runtime-host-loading.host-taxonomy]]
Aspen MUST classify runtime units by an explicit host kind before resolving or starting their executable artifact, and OCI image identity SHALL be modeled as an artifact/lowering input rather than a production host boundary.
ID: [r[runtime-host-loading.host-taxonomy]]

#### Scenario: OCI image artifact selected [r[runtime-host-loading.host-taxonomy.oci-container]]
OCI runtime declarations MUST use immutable image identity and an isolated lowering target rather than selecting a plain container host as the production boundary.
ID: [r[runtime-host-loading.host-taxonomy.oci-container]]
- GIVEN a runtime unit declares an OCI image artifact
- WHEN the runtime resolves the unit for production execution
- THEN the declaration SHALL use an immutable image digest rather than a mutable tag for accepted execution
- AND it SHALL state the lowering target, entrypoint, arguments, mounts, environment handles, network policy, and capability bindings needed before start
- AND the selected production host boundary SHALL be `MicroVm`, `Hyperlight`, `Wasm`, or a VM-backed guest profile such as `Unikernel { HermitOs }` rather than a Podman/Docker-style host container

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

### Requirement: Content-Addressed Dynamic Artifact Loading [r[runtime-host-loading.dynamic-artifacts]]
Aspen MUST verify dynamic runtime artifacts by content identity before instantiating WASM modules, starting Hyperlight units, lowering OCI images into isolated hosts, launching microVM guests, or launching external native processes.
ID: [r[runtime-host-loading.dynamic-artifacts]]

#### Scenario: OCI image verified before isolated lowering [r[runtime-host-loading.dynamic-artifacts.oci-verified]]
OCI image admission MUST verify immutable image identity and provenance before deriving rootfs, program, guest, or component artifacts for an isolated host.
ID: [r[runtime-host-loading.dynamic-artifacts.oci-verified]]
- GIVEN an OCI-backed runtime unit declares an image digest
- WHEN the admission/lowering planner prepares a plan for `MicroVm`, `Hyperlight`, `Wasm`, or a VM-backed guest profile such as `Unikernel { HermitOs }`
- THEN the planner SHALL resolve and verify the immutable image digest and signature/provenance policy before deriving executable artifacts
- AND the selected runner SHALL re-verify the derived rootfs, program, or guest artifact identity before launch
- AND mutable tags SHALL NOT be accepted as the durable execution identity
- AND layer/rootfs/program/guest materialization SHALL be recorded as bounded receipt data without raw credentials

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

### Requirement: Hyperlight Runtime Runner [r[runtime-host-loading.hyperlight-runner]]
Aspen MUST provide a Hyperlight runner contract for isolated runtime units that are compatible with the Hyperlight host boundary.

#### Scenario: Hyperlight runner capability is advertised [r[runtime-host-loading.hyperlight-runner.capability]]
- GIVEN an Aspen node includes a compatible Hyperlight runtime
- WHEN the node reports runtime runner capabilities
- THEN it SHALL advertise Hyperlight support, runner version, supported ABI profiles, resource limits, and artifact profiles

#### Scenario: Hyperlight artifact is verified before start [r[runtime-host-loading.hyperlight-runner.artifact-verification]]
- GIVEN a runtime unit declares a Hyperlight image or program artifact
- WHEN the runner prepares to start the unit
- THEN it SHALL verify content identity, ABI compatibility, and resource policy before exposing host calls or capability handles

#### Scenario: Host ABI exposes only declared capabilities [r[runtime-host-loading.hyperlight-runner.capability-binding]]
- GIVEN a Hyperlight unit requests Aspen substrate access
- WHEN the runner constructs the host ABI
- THEN it SHALL expose only declared capability-scoped handles for KV, blob, logging, metrics, timers, routes, or outputs
- AND it SHALL deny undeclared devices, sockets, files, environment variables, network access, routes, secrets, and host calls with bounded diagnostics

#### Scenario: Hyperlight admission fails closed [r[runtime-host-loading.hyperlight-runner.fail-closed]]
- GIVEN a Hyperlight unit has an invalid artifact, unsupported ABI, missing runner capability, or denied capability binding
- WHEN admission evaluates the unit
- THEN Aspen SHALL reject the assignment before start and SHALL emit a redacted rejection receipt

#### Scenario: Hyperlight output is receipt-backed [r[runtime-host-loading.hyperlight-runner.outputs]]
- GIVEN a Hyperlight unit exits or emits declared outputs
- WHEN the runner finalizes the attempt
- THEN outputs SHALL be stored as Aspen-approved artifacts or receipt fields and SHALL include the verified input artifact identity and runner identity

### Requirement: MicroVM Runtime Runner [r[runtime-host-loading.microvm-runner]]
Aspen MUST provide a node-local microVM runner contract for isolated runtime units that require a VM or microVM host boundary.

#### Scenario: Runner capability is advertised [r[runtime-host-loading.microvm-runner.capability]]
- GIVEN an Aspen node can launch a supported microVM engine such as Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm, or equivalent
- WHEN the node reports runtime runner capabilities
- THEN it SHALL advertise the supported engine, virtualization backend, resource limits, supported guest artifact profiles, and runner version

#### Scenario: Assignment fails closed without compatible runner [r[runtime-host-loading.microvm-runner.fail-closed]]
- GIVEN a runtime unit requires a microVM host boundary
- WHEN the scheduler or node admission evaluates an assignment
- THEN admission SHALL fail closed unless the selected node advertises a compatible microVM runner and sufficient declared resources

#### Scenario: Guest artifacts are prepared before launch [r[runtime-host-loading.microvm-runner.artifact-prep]]
- GIVEN a microVM runtime unit declares kernel, initrd, rootfs, disk, or guest-image artifacts
- WHEN the runner prepares the unit
- THEN it SHALL verify content identity before launch and SHALL record the verified artifact identities in the launch receipt

#### Scenario: Launch bindings deny ambient authority [r[runtime-host-loading.microvm-runner.launch-bindings]]
- GIVEN a microVM unit requests mounts, block devices, network interfaces, vsock channels, environment-like metadata, or capability handles
- WHEN the runner prepares the launch
- THEN it SHALL attach only declared and authorized bindings and SHALL deny undeclared devices, host paths, sockets, networks, secrets, and ambient host access before boot

#### Scenario: Runner records lifecycle receipts [r[runtime-host-loading.microvm-runner.receipts]]
- GIVEN a microVM unit starts, stops, fails, times out, or is killed
- WHEN the runner observes the lifecycle transition
- THEN it SHALL emit secret-safe receipts containing unit identity, assigned node, engine, attempt, lifecycle state, resource summary, artifact identities, and redacted handle summary

#### Scenario: Logs and outputs become artifacts [r[runtime-host-loading.microvm-runner.outputs]]
- GIVEN a microVM unit produces serial logs, stdout/stderr streams, disk outputs, or declared result artifacts
- WHEN the unit exits or checkpoints output
- THEN the runner SHALL persist bounded logs and outputs as Aspen-approved artifacts or explicit receipt fields without leaking raw secrets

### Requirement: WASM Runtime Service Host [r[runtime-host-loading.wasm-service-host]]
Aspen MUST provide a bounded WASM host contract for deterministic hooks, policies, plugins, and compatible runtime service fragments.

#### Scenario: WASM admission validates ABI and limits [r[runtime-host-loading.wasm-service-host.admission]]
- GIVEN a runtime declaration references a WASM module
- WHEN the WASM host admits the module
- THEN it SHALL verify module content identity, ABI version, entrypoint, fuel, memory, timeout, and capability policy before instantiation

#### Scenario: Host functions are capability-scoped [r[runtime-host-loading.wasm-service-host.capability-scoped-functions]]
- GIVEN a WASM module requests Aspen host functions
- WHEN the module is instantiated
- THEN the host SHALL expose only declared capability-scoped functions and SHALL deny undeclared KV, blob, route, network, clock, or secret access

#### Scenario: Deterministic extension mode is bounded [r[runtime-host-loading.wasm-service-host.deterministic-extension]]
- GIVEN a WASM module is used as a policy hook or service extension
- WHEN it executes inside a deterministic extension mode
- THEN the host SHALL bound fuel, memory, time, input size, output size, and ambient effects

#### Scenario: WASM service fragment declares route ownership [r[runtime-host-loading.wasm-service-host.service-fragment-routes]]
- GIVEN a WASM artifact is admitted as a runtime service fragment rather than a plugin
- WHEN it declares route ownership
- THEN the WASM host SHALL validate WASM-specific ABI and capability prerequisites
- AND `runtime-service-core` SHALL remain authoritative for route ownership, route-conflict resolution, and route-registration receipts before activation

#### Scenario: WASM failure emits redacted receipt [r[runtime-host-loading.wasm-service-host.failure-receipt]]
- GIVEN WASM validation, instantiation, execution, or host-call authorization fails
- WHEN the host records the failure
- THEN it SHALL emit a receipt with module identity, ABI, failure class, bounded diagnostics, and redacted capability summary

### Requirement: OCI Artifact Lowering [r[runtime-host-loading.oci-lowering]]
Aspen MUST treat OCI images as content-addressed artifact inputs that lower into isolated runtime host boundaries rather than as production host boundaries themselves.
ID: [r[runtime-host-loading.oci-lowering]]

#### Scenario: OCI image lowers to microVM by default [r[runtime-host-loading.oci-lowering.microvm-default]]
Production OCI execution MUST use a microVM-backed lowering target by default for tenant, CI, remote application, and risky adapter workloads.
ID: [r[runtime-host-loading.oci-lowering.microvm-default]]
- GIVEN a runtime unit declares an OCI image artifact for a tenant, CI, remote application, or risky adapter workload
- WHEN the runtime admits the unit for production execution
- THEN the admission plan SHALL select a `MicroVm` host boundary unless node policy selects a stronger or narrower compatible isolated target
- AND the plan SHALL reject ordinary host-container execution as the default boundary

#### Scenario: OCI-backed service spec declares lowering contract [r[runtime-host-loading.oci-lowering.service-spec-contract]]
OCI-backed service specs MUST carry the runtime service contract fields needed before lowering.
ID: [r[runtime-host-loading.oci-lowering.service-spec-contract]]
- GIVEN a runtime service spec references an OCI image artifact
- WHEN production admission evaluates the service spec
- THEN it SHALL include service identity, artifact identity, host-loading reference or lowering target, resources, placement, capability bindings, route policy when applicable, health policy, restart policy, upgrade policy, and receipt policy before lowering can proceed

#### Scenario: OCI image lowers to compatible specialized host [r[runtime-host-loading.oci-lowering.specialized-target]]
OCI artifact lowering MUST be explicit when the target is Hyperlight, WASM, or a unikernel profile.
ID: [r[runtime-host-loading.oci-lowering.specialized-target]]
- GIVEN an OCI image contains or can be rebuilt into a Hyperlight program, WASM component, or unikernel guest artifact
- WHEN the runtime selects that specialized target
- THEN the lowering plan SHALL record the transformation or rebuild provenance, derived artifact identity, selected host kind, and unsupported-feature diagnostics if lowering fails

#### Scenario: OCI lowering receipt preserves original and derived identities [r[runtime-host-loading.oci-lowering.receipt]]
OCI lowering receipts MUST identify both the source OCI image and the isolated execution artifact without exposing secrets.
ID: [r[runtime-host-loading.oci-lowering.receipt]]
- GIVEN an OCI-backed runtime unit is admitted and started through an isolated host boundary
- WHEN the runtime emits the start or completion receipt
- THEN the receipt SHALL include the original immutable OCI digest, selected lowering target, derived rootfs/program/guest artifact hashes, runner identity, and bounded redacted handle summary
- AND it SHALL NOT include registry credentials, raw environment secrets, mutable tags as durable identity, or ambient host paths

#### Scenario: Plain container runner is dev or unsafe only [r[runtime-host-loading.oci-lowering.raw-container-dev-only]]
Plain Podman/Docker-style host-container execution MUST NOT be part of the default production runtime contract.
ID: [r[runtime-host-loading.oci-lowering.raw-container-dev-only]]
- GIVEN a future local runner supports ordinary host-container execution for development or trusted operator smokes
- WHEN a production runtime declaration requests that raw container runner
- THEN Aspen SHALL reject the declaration unless explicit dev/unsafe policy is enabled
- AND any accepted dev/unsafe run SHALL be marked in receipts as weaker isolation than microVM, Hyperlight, WASM, or unikernel execution

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class, and row promotion MUST require runnable product-path evidence for the specific host kind being promoted.

#### Scenario: Missing host rows remain gaps [r[runtime-host-loading.e2e-matrix.gap-labels]]
- GIVEN WASM runner, OCI lowering, Hyperlight, or Hermit host classes lack full Aspen-spawned execution tests
- WHEN the runtime-host matrix is reviewed
- THEN those rows SHALL remain labeled as gaps or future work until an E2E suite executes through the real Aspen runtime path and produces product-visible output or receipt evidence
- AND metadata-only rows SHALL be non-runnable evidence inventory entries rather than substitutes for product execution tests
- AND promoting one host class SHALL NOT imply readiness for the remaining metadata-only host classes

#### Scenario: Metadata rows carry explicit host proof labels [r[runtime-host-loading.e2e-matrix.metadata-labels]]
- GIVEN the harness inventory records a runtime-host row
- WHEN the row is exported for operators or CI tooling
- THEN it SHALL include the runtime host kind, proof level, and support status when the row is part of the runtime-host matrix
- AND metadata-only gap rows SHALL include a human-readable gap reason and no runnable build target
- AND promoted runnable rows SHALL name their target command or flake attribute and the proof markers operators must require before citing readiness

### Requirement: WASM Runtime Host E2E Promotion [r[runtime-host-loading.wasm-e2e-promotion]]
Aspen MUST promote the WASM runtime-host matrix row only when a runnable suite executes a WASM unit through the real Aspen runtime path and records product-visible output or receipt evidence.

#### Scenario: WASM row uses product runtime path [r[runtime-host-loading.wasm-e2e-promotion.product-path]]
- GIVEN the `runtime-host-wasm-gap` metadata row is being promoted
- WHEN Aspen publishes the replacement row as runnable evidence
- THEN the suite SHALL start Aspen with the WASM runtime host capability enabled
- AND it SHALL activate or submit a WASM runtime unit through product RPC, CLI, or orchestration APIs rather than calling `aspen-runtime-core` helpers directly
- AND it SHALL observe lifecycle completion through Aspen-visible state, output, or receipt data

#### Scenario: WASM proof markers are explicit [r[runtime-host-loading.wasm-e2e-promotion.proof-markers]]
- GIVEN the runnable WASM suite completes successfully
- WHEN the evidence log or receipt is reviewed
- THEN it SHALL include module identity, runner or host identity, lifecycle state, and bounded output summary
- AND it SHALL include a stable marker that distinguishes real WASM execution from plugin installation, plugin reload, or admission-only validation

#### Scenario: WASM receipts remain secret-safe [r[runtime-host-loading.wasm-e2e-promotion.secret-safe-receipts]]
- GIVEN the WASM runtime unit receives capability handles, configuration, logs, or output bindings
- WHEN the suite records logs, receipts, manifests, or artifacts
- THEN the evidence SHALL contain only opaque handles, hashes, redacted summaries, or artifact references for sensitive material
- AND it SHALL NOT contain raw tokens, tickets, private keys, cluster cookies, connection strings, or secret values

#### Scenario: Metadata-only paths do not satisfy promotion [r[runtime-host-loading.wasm-e2e-promotion.no-overclaim]]
- GIVEN runtime-core model tests, WASM admission tests, plugin install/reload plumbing, or harness inventory metadata pass
- WHEN the runtime-host matrix is evaluated
- THEN those checks SHALL NOT be labeled `aspen-spawned-execution` for WASM unless the runnable suite also executes the WASM unit through the Aspen runtime path and captures product-visible evidence
