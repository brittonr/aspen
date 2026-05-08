## ADDED Requirements

### Requirement: Canonical Runtime Service Contract [r[runtime-service-core.canonical-contract]]

Aspen MUST define a canonical runtime-service contract that connects service declarations, runtime-host loading, job/plugin execution backends, deploy actions, route ownership, health, and receipts without conflating validation with activation.

#### Scenario: Service contract links execution backend [r[runtime-service-core.canonical-contract.execution-backend]]

- GIVEN a runtime service spec references a native built-in, WASM, Hyperlight, microVM, Hermit/Uhyve, external process, or deploy-backed execution target
- WHEN the service contract is validated
- THEN it SHALL record service identity, generation, host-loading reference, artifact identity, backend kind, capability bindings, resource policy, and receipt policy
- AND validation SHALL NOT by itself claim the service is admitted, started, healthy, or route-active

#### Scenario: Receipts correlate service, job, plugin, and deploy events [r[runtime-service-core.canonical-contract.receipt-correlation]]

- GIVEN a service instance is started through a job worker, plugin runner, native built-in wrapper, or deploy executor
- WHEN lifecycle or health receipts are emitted
- THEN receipts SHALL carry stable correlation identifiers for service, instance, generation, backend execution, artifact identity, and route ownership without log scraping

#### Scenario: Route activation waits for health boundary [r[runtime-service-core.canonical-contract.route-health-boundary]]

- GIVEN a service declares one or more route families
- WHEN execution starts but health is unknown or failed
- THEN route activation SHALL remain pending or be withdrawn until policy-specific health criteria pass
- AND operator receipts SHALL distinguish route-declared, route-pending, route-active, and route-withdrawn states
