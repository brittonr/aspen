## ADDED Requirements

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

## MODIFIED Requirements

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
