## ADDED Requirements

### Requirement: Remaining Runtime-Host Row Promotion Contract [r[runtime-host-loading.remaining-row-promotion]]
Aspen MUST promote any remaining or newly introduced metadata-only runtime-host matrix row only through a row-specific product-path proof package.

#### Scenario: Promotion names row and host boundary [r[runtime-host-loading.remaining-row-promotion.named-boundary]]
- GIVEN a runtime-host matrix row is metadata-only, future-work, or otherwise not yet product-path proven
- WHEN work begins to promote that row
- THEN the OpenSpec change SHALL name the row id, host kind, artifact profile, runtime capability, and selected product orchestration path
- AND it SHALL state which existing model, package, admission, or direct-worker tests are prerequisite guardrails rather than readiness evidence

#### Scenario: Promotion requires product orchestration [r[runtime-host-loading.remaining-row-promotion.product-path]]
- GIVEN a row is promoted to runtime-host readiness
- WHEN the row is cited as runnable evidence
- THEN the proof SHALL submit, start, or reconcile the unit through Aspen product RPC, CLI, runtime reconciliation, `JobManager`/`WorkerPool`, or an equivalent product-facing orchestration seam
- AND direct runtime-core helper calls, direct worker invocations, package builds, plugin install/reload, or inventory generation SHALL NOT satisfy the row by themselves

#### Scenario: Promotion receipts are explicit and secret-safe [r[runtime-host-loading.remaining-row-promotion.receipts]]
- GIVEN a promoted row proof completes
- WHEN operators inspect the evidence log, receipt, or harness metadata
- THEN it SHALL include host kind, artifact identity, runner identity, lifecycle status, bounded output summary, and a stable proof marker for the selected row
- AND it SHALL NOT include raw tokens, tickets, private keys, cluster cookies, connection strings, or secret values

## MODIFIED Requirements

### Requirement: Runtime Host E2E Matrix [r[runtime-host-loading.e2e-matrix]]
Aspen MUST maintain explicit runtime-host E2E coverage metadata that distinguishes model/admission tests, real host execution tests, and Aspen-spawned execution tests for each supported host class, and row promotion MUST require runnable product-path evidence for the specific host kind being promoted.

#### Scenario: Metadata rows remain explicit gaps until proven [r[runtime-host-loading.e2e-matrix.metadata-gap-boundary]]
- GIVEN a runtime-host matrix row has only model tests, admission tests, package builds, direct-worker tests, documentation, or harness metadata
- WHEN the runtime-host matrix is reviewed
- THEN that row SHALL remain labeled as a gap, future-work, or prerequisite-only row until a product-path suite executes the selected host kind and records product-visible output or receipt evidence
- AND promoting one host kind SHALL NOT imply readiness for any other metadata-only or future-work host kind
