## ADDED Requirements

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

## MODIFIED Requirements

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
