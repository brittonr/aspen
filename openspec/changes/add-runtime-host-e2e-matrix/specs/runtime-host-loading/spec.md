## ADDED Requirements

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class.

#### Scenario: Host row names proof level [r[runtime-host-loading.e2e-matrix.proof-level]]
- GIVEN a runtime host class such as microVM, WASM, OCI lowering, Hyperlight, or Hermit
- WHEN Aspen publishes test-harness metadata or operator-facing verification guidance for that host class
- THEN the metadata SHALL state whether the suite proves model-only behavior, real host execution, or Aspen-spawned execution
- AND it SHALL NOT describe model/admission tests as full Aspen-spawned execution evidence

#### Scenario: MicroVM row uses CloudHypervisorWorker path [r[runtime-host-loading.e2e-matrix.microvm-ci-vm]]
- GIVEN the existing CI VM worker path can start Cloud Hypervisor guests from Aspen node configuration
- WHEN the runtime-host matrix lists the first microVM Aspen-spawned proof
- THEN it SHALL reference a suite that starts Aspen, enables the `CloudHypervisorWorker` path, submits `ci_vm` work through Aspen, and observes guest-backed job completion or receipt evidence
- AND it SHALL mark nested KVM and impure execution requirements explicitly

#### Scenario: Missing host rows remain gaps [r[runtime-host-loading.e2e-matrix.gap-labels]]
- GIVEN WASM runner, OCI lowering, Hyperlight, or Hermit host classes lack full Aspen-spawned execution tests
- WHEN the runtime-host matrix is reviewed
- THEN those rows SHALL remain labeled as gaps or future work until an E2E suite executes through the real Aspen runtime path and produces product-visible output or receipt evidence

#### Scenario: Secret-safe receipt evidence [r[runtime-host-loading.e2e-matrix.secret-safe-evidence]]
- GIVEN a runtime-host E2E suite records logs, receipts, manifests, or output artifacts
- WHEN the evidence is persisted or shown to operators
- THEN it SHALL include bounded host identity, runner identity, job or unit identity, lifecycle state, and output summaries without raw tokens, tickets, private keys, cluster cookies, connection strings, or registry credentials
