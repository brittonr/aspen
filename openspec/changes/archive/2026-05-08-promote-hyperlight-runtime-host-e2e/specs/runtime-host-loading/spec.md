## ADDED Requirements

### Requirement: Hyperlight Runtime Host E2E Promotion [r[runtime-host-loading.hyperlight-e2e-promotion]]
Aspen MUST promote the Hyperlight runtime-host matrix row only when a runnable suite executes a Hyperlight guest through the real Aspen runtime path and records product-visible output or receipt evidence.

#### Scenario: Hyperlight row uses product runtime path [r[runtime-host-loading.hyperlight-e2e-promotion.product-path]]
- GIVEN the `runtime-host-hyperlight-gap` metadata row is being promoted
- WHEN Aspen publishes the replacement row as runnable evidence
- THEN the suite SHALL submit or activate a Hyperlight runtime unit through product job orchestration or node worker registration rather than only constructing `HyperlightWorker` or calling runtime helpers directly
- AND it SHALL retrieve or build the guest artifact through a declared Aspen-owned artifact path such as blob-backed `vm_execute` input
- AND it SHALL observe lifecycle completion through Aspen-visible job state, output, or receipt data

#### Scenario: Hyperlight proof markers are explicit [r[runtime-host-loading.hyperlight-e2e-promotion.proof-markers]]
- GIVEN the runnable Hyperlight suite completes successfully
- WHEN the evidence log or receipt is reviewed
- THEN it SHALL include guest artifact identity, runner or host identity, lifecycle state, exit status, duration or bounded output summary, and the stable marker `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED` or a documented successor
- AND it SHALL include a separate guard marker that distinguishes real Hyperlight execution from worker construction, payload serialization, package builds, ignored/manual examples, or direct worker-only validation

#### Scenario: Hyperlight receipts remain secret-safe [r[runtime-host-loading.hyperlight-e2e-promotion.secret-safe-receipts]]
- GIVEN the Hyperlight runtime unit receives input, configuration, logs, handles, or output bindings
- WHEN the suite records logs, receipts, manifests, or artifacts
- THEN the evidence SHALL contain only opaque handles, hashes, redacted summaries, bounded output, or artifact references for sensitive material
- AND it SHALL NOT contain raw tokens, tickets, private keys, cluster cookies, registry credentials, host-private paths, connection strings, or secret values

#### Scenario: Metadata-only paths do not satisfy Hyperlight promotion [r[runtime-host-loading.hyperlight-e2e-promotion.no-overclaim]]
- GIVEN Hyperlight profile/admission tests, payload serialization tests, worker construction tests, package builds, examples, ignored tests, or harness inventory metadata pass
- WHEN the runtime-host matrix is evaluated
- THEN those checks SHALL NOT be labeled `aspen-spawned-execution` for Hyperlight unless the runnable suite also executes the Hyperlight guest through the Aspen runtime path and captures product-visible evidence

## MODIFIED Requirements

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class, and row promotion MUST require runnable product-path evidence for the specific host kind being promoted.

#### Scenario: Missing host rows remain gaps [r[runtime-host-loading.e2e-matrix.gap-labels]]
- GIVEN OCI lowering, Hyperlight, or Hermit host classes lack full Aspen-spawned execution tests
- WHEN the runtime-host matrix is reviewed
- THEN those rows SHALL remain labeled as gaps or future work until an E2E suite executes through the real Aspen runtime path and produces product-visible output or receipt evidence
- AND metadata-only rows SHALL be non-runnable evidence inventory entries rather than substitutes for product execution tests
- AND promoting one host class SHALL NOT imply readiness for the remaining metadata-only host classes
