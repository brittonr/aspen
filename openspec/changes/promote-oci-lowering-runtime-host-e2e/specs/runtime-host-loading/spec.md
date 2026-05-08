## ADDED Requirements

### Requirement: OCI Lowering Runtime Host E2E Promotion [r[runtime-host-loading.oci-lowering-e2e-promotion]]
Aspen MUST promote the OCI lowering runtime-host matrix row only when a runnable suite ingests immutable OCI artifact identity, lowers it into a supported isolated runtime host, executes the derived artifact through the real Aspen runtime path, and records product-visible output or receipt evidence.

#### Scenario: OCI lowering row links source artifact to isolated execution [r[runtime-host-loading.oci-lowering-e2e-promotion.product-path]]
- GIVEN the `runtime-host-oci-lowering-gap` metadata row is being promoted
- WHEN Aspen publishes the replacement row as runnable evidence
- THEN the suite SHALL start from an immutable OCI artifact identity such as a `sha256:` image digest rather than a mutable tag alone
- AND it SHALL lower that OCI artifact into a declared isolated target host artifact for `MicroVm`, `Wasm`, `Hyperlight`, or a VM-backed unikernel profile rather than executing it as a raw host container
- AND it SHALL submit or activate the derived artifact through Aspen product orchestration for the selected target host
- AND it SHALL observe lifecycle completion through Aspen-visible job, runtime state, output, or receipt data

#### Scenario: OCI lowering proof markers are explicit [r[runtime-host-loading.oci-lowering-e2e-promotion.proof-markers]]
- GIVEN the runnable OCI lowering suite completes successfully
- WHEN the evidence log or receipt is reviewed
- THEN it SHALL include source OCI digest, selected target host, derived artifact identity, runner or host identity, lifecycle state, exit status, duration or bounded output summary, and the stable marker `ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED` or a documented successor
- AND it SHALL include a separate guard marker that distinguishes real OCI-lowered execution from plan construction, admission/model tests, image metadata parsing, package builds, mutable tag resolution, raw-container smokes, or target-host execution without OCI source provenance

#### Scenario: OCI lowering receipts remain secret-safe [r[runtime-host-loading.oci-lowering-e2e-promotion.secret-safe-receipts]]
- GIVEN the OCI artifact or lowered runtime unit receives registry handles, configuration, logs, capability handles, or output bindings
- WHEN the suite records logs, receipts, manifests, or artifacts
- THEN the evidence SHALL contain only immutable digests, opaque handles, hashes, redacted summaries, bounded output, derived artifact references, or target-host receipt references for sensitive material
- AND it SHALL NOT contain registry credentials, raw environment secrets, mutable tags as durable identity, ambient host paths, raw tokens, tickets, private keys, cluster cookies, connection strings, or secret values

#### Scenario: Metadata-only and raw-container paths do not satisfy OCI lowering promotion [r[runtime-host-loading.oci-lowering-e2e-promotion.no-overclaim]]
- GIVEN OCI lowering admission tests, `OciLoweringPlan`/`OciLoweringReceipt` model tests, registry metadata checks, package builds, raw Podman/Docker-style container execution, or harness inventory metadata pass
- WHEN the runtime-host matrix is evaluated
- THEN those checks SHALL NOT be labeled `aspen-spawned-execution` for OCI lowering unless the runnable suite also lowers immutable OCI input into a supported isolated target host, executes the derived artifact through the Aspen runtime path, and captures product-visible evidence linking both identities

## MODIFIED Requirements

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class, and row promotion MUST require runnable product-path evidence for the specific host kind being promoted.

#### Scenario: Missing host rows remain gaps [r[runtime-host-loading.e2e-matrix.gap-labels]]
- GIVEN OCI lowering or Hermit host classes lack full Aspen-spawned execution tests
- WHEN the runtime-host matrix is reviewed
- THEN those rows SHALL remain labeled as gaps or future work until an E2E suite executes through the real Aspen runtime path and produces product-visible output or receipt evidence
- AND metadata-only rows SHALL be non-runnable evidence inventory entries rather than substitutes for product execution tests
- AND promoting one host class SHALL NOT imply readiness for the remaining metadata-only host classes
