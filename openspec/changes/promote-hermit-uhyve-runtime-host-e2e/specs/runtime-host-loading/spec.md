## ADDED Requirements

### Requirement: Hermit/Uhyve Runtime Host E2E Promotion [r[runtime-host-loading.hermit-uhyve-e2e-promotion]]
Aspen MUST promote the Hermit runtime-host matrix row only when a runnable suite executes a Hermit unikernel guest through a real Aspen orchestration path and records product-visible output or receipt evidence from the selected Uhyve runner.

#### Scenario: Hermit row uses Uhyve product runtime path [r[runtime-host-loading.hermit-uhyve-e2e-promotion.product-path]]
- GIVEN the `runtime-host-hermit-gap` metadata row is being promoted
- WHEN Aspen publishes the replacement row as runnable evidence
- THEN the suite SHALL submit or activate a Hermit unikernel runtime unit through product job orchestration or node worker registration rather than only invoking `uhyve` from a shell or calling runtime-core helpers directly
- AND it SHALL retrieve or build the Hermit image through a declared Aspen-owned artifact path such as blob-backed job input
- AND it SHALL select `MicroVmEngine::Uhyve` or a documented successor compatible with `HermitLaunchProfileKind::Uhyve`
- AND it SHALL observe lifecycle completion through Aspen-visible job state, output, or receipt data

#### Scenario: Hermit/Uhyve proof markers are explicit [r[runtime-host-loading.hermit-uhyve-e2e-promotion.proof-markers]]
- GIVEN the runnable Hermit/Uhyve suite completes successfully
- WHEN the evidence log or receipt is reviewed
- THEN it SHALL include Hermit image hash, runner or host identity, Uhyve engine identity, lifecycle state, exit status, duration or bounded serial output summary, and the stable marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` or a documented successor
- AND it SHALL include a separate guard marker that distinguishes real Hermit/Uhyve execution from profile admission, receipt model tests, package builds, direct `uhyve` shell commands, ignored/manual examples, or direct worker-only validation

#### Scenario: Hermit/Uhyve receipts remain secret-safe [r[runtime-host-loading.hermit-uhyve-e2e-promotion.secret-safe-receipts]]
- GIVEN the Hermit runtime unit receives boot arguments, capability handles, loader metadata, logs, channels, or output bindings
- WHEN the suite records logs, receipts, manifests, or artifacts
- THEN the evidence SHALL contain only opaque handles, immutable hashes, redacted summaries, bounded serial output, or artifact references for sensitive material
- AND it SHALL NOT contain raw boot secrets, tokens, tickets, private keys, cluster cookies, registry credentials, host-private paths, connection strings, or secret values

#### Scenario: Metadata-only and direct Uhyve paths do not satisfy Hermit promotion [r[runtime-host-loading.hermit-uhyve-e2e-promotion.no-overclaim]]
- GIVEN Hermit profile/admission tests, `HermitProfileReceipt` model tests, package builds, direct `uhyve` execution, skipped or ignored tests not run in a capable environment, or harness inventory metadata pass
- WHEN the runtime-host matrix is evaluated
- THEN those checks SHALL NOT be labeled `aspen-spawned-execution` for Hermit unless the runnable suite also executes the Hermit image through the Aspen runtime path and captures product-visible evidence

## MODIFIED Requirements

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class, and row promotion MUST require runnable product-path evidence for the specific host kind being promoted.

#### Scenario: Missing host rows remain gaps [r[runtime-host-loading.e2e-matrix.gap-labels]]
- GIVEN Hermit host classes lack full Aspen-spawned execution tests
- WHEN the runtime-host matrix is reviewed
- THEN those rows SHALL remain labeled as gaps or future work until an E2E suite executes through the real Aspen runtime path and produces product-visible output or receipt evidence
- AND metadata-only rows SHALL be non-runnable evidence inventory entries rather than substitutes for product execution tests
- AND promoting one host class SHALL NOT imply readiness for the remaining metadata-only host classes
