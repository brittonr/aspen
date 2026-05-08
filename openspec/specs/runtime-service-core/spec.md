# runtime-service-core Specification

## Purpose

This spec defines Aspen's portable runtime-service core model: application ownership references, service specs and instances, linked native built-in service declarations, route ownership, secret-safe receipts, and bounded policy/invariant behavior. It is intentionally a model/contract layer below future runtime-service reconciliation and app installation work.
## Requirements
### Requirement: Runtime Application Ownership References [r[runtime-service-core.application-ownership]]
Aspen MUST define minimal application ownership references for grouping runtime services, routes, capability requests, and receipts without implementing full application installation in this change.

#### Scenario: Service belongs to application identity [r[runtime-service-core.application-ownership.service-belongs-to-app]]
- GIVEN a runtime service spec is declared by a first-party or future installed application
- WHEN the runtime service model records ownership
- THEN it SHALL include stable application identity, service identity, generation, route namespace, and receipt ownership reference

#### Scenario: Full app lifecycle is deferred [r[runtime-service-core.application-ownership.lifecycle-deferred]]
- GIVEN a service spec contains an application ownership reference
- WHEN the service core validates the model
- THEN the reference SHALL NOT by itself imply app install, app upgrade, migration execution, or dynamic marketplace support

### Requirement: Runtime Service Core Model [r[runtime-service-core.model]]
Aspen MUST define a portable runtime service model for durable application units above the Raft/KV substrate and below first-party or user-facing applications.

#### Scenario: Service spec declares runtime contract [r[runtime-service-core.model.service-spec]]
- GIVEN an Aspen application declares a long-lived service
- WHEN the declaration is accepted by the runtime service core
- THEN the service spec SHALL include stable service identity, artifact identity, host-loading reference, desired replicas or singleton policy, placement hints, resources, capability bindings, route declarations, health policy, restart policy, upgrade policy, and receipt policy

#### Scenario: Service instance tracks concrete assignment model [r[runtime-service-core.model.service-instance]]
- GIVEN a service spec will later be reconciled into a concrete runtime instance
- WHEN the portable model records the instance state
- THEN the runtime SHALL track service identity, instance identity, generation, optional assigned node, lifecycle state, health state, lease epoch, heartbeat timestamp, active routes, and last receipt identity
- AND the model SHALL NOT require a full distributed scheduler to exist before service instances can be represented

#### Scenario: Model remains portable [r[runtime-service-core.model.portable]]
- GIVEN the runtime service model is used by planning, tests, or future non-node tools
- WHEN model values are constructed, serialized, or validated
- THEN the model SHALL avoid direct process spawning, network I/O, filesystem I/O, secret material, and runtime-specific handles

#### Scenario: Host loading reference is not activation [r[runtime-service-core.model.host-loading-not-activation]]
- GIVEN a service spec references a valid runtime host-loading declaration or artifact profile
- WHEN the service core validates the service model
- THEN successful host-loading validation SHALL NOT by itself imply service admission, route activation, running status, or healthy status

### Requirement: Native Built-In Service Registry [r[runtime-service-core.native-built-in-registry]]
Aspen MUST provide a linked native built-in service registry for first-party services without using in-process dynamic native plugin loading as the default service mechanism.

#### Scenario: Built-in service exposes manifest [r[runtime-service-core.native-built-in-registry.manifest]]
- GIVEN the node binary contains a first-party service such as Forge
- WHEN the runtime asks the built-in registry for available services
- THEN the registry SHALL return service manifests with service identity, built-in artifact identity, host kind, declared routes, required capability handles, health policy, and receipt schema

#### Scenario: Dynamic native plugin is not the built-in path [r[runtime-service-core.native-built-in-registry.no-dlopen]]
- GIVEN a service attempts to register through an in-process dynamic native library boundary
- WHEN default runtime service admission evaluates the declaration
- THEN admission SHALL reject it unless a future OpenSpec explicitly accepts that unsafe ABI boundary

### Requirement: Runtime Route Ownership [r[runtime-service-core.routes]]
Aspen MUST model route ownership as a runtime service declaration rather than as only ad hoc handler registration side effects.

#### Scenario: Service routes are declared before activation [r[runtime-service-core.routes.declared-before-activation]]
- GIVEN a runtime service declares route families it owns
- WHEN the service transitions toward running
- THEN route declarations SHALL be validated against service identity, capability bindings, and route conflicts before the service is reported healthy

#### Scenario: Route registration emits receipt [r[runtime-service-core.routes.receipt]]
- GIVEN a service route family is registered or removed
- WHEN the runtime updates the route table
- THEN it SHALL emit a receipt containing service identity, route identity, generation, action, node identity when applicable, and redacted capability summary

### Requirement: Runtime Service Receipts [r[runtime-service-core.receipts]]
Aspen MUST emit secret-safe runtime service receipts for lifecycle, route, health, start, stop, failure, and upgrade decisions.

#### Scenario: Lifecycle receipt records transition [r[runtime-service-core.receipts.lifecycle-transition]]
- GIVEN a service instance changes lifecycle state
- WHEN the runtime records the transition
- THEN the receipt SHALL include service identity, instance identity, generation, prior state, next state, reason, timestamp or logical clock, node identity when applicable, and artifact identity

#### Scenario: Health receipt records transition [r[runtime-service-core.receipts.health-transition]]
- GIVEN a service instance health state changes or is observed as degraded
- WHEN the runtime records the health event
- THEN the receipt SHALL include service identity, instance identity, generation, prior health when known, next health, bounded reason, route impact when applicable, and redacted capability summary

#### Scenario: Receipt redacts secrets [r[runtime-service-core.receipts.secret-redaction]]
- GIVEN service startup uses tokens, tickets, private keys, cluster cookies, connection strings, kernel arguments, environment variables, or capability handles
- WHEN a runtime receipt, manifest, log summary, or operator report is emitted
- THEN it SHALL NOT include raw secret material and SHALL include only opaque handles, content hashes, or redacted summaries

### Requirement: Forge Native Runtime Service Slice [r[runtime-service-core.forge-slice]]
Aspen MUST use Forge as the first native built-in service migration target by wrapping existing Forge startup and handler behavior in a runtime service contract.

#### Scenario: Forge exposes runtime service manifest [r[runtime-service-core.forge-slice.manifest]]
- GIVEN the node binary includes Forge support
- WHEN the runtime service registry lists built-in services
- THEN Forge SHALL expose a service manifest with built-in artifact identity, declared Forge route families, required KV/blob/gossip/execution capability handles, health checks, and receipt schema

#### Scenario: Forge wrapper preserves internals [r[runtime-service-core.forge-slice.preserve-internals]]
- GIVEN current Forge startup constructs `ForgeNode` and registers Forge handlers directly
- WHEN the first runtime service slice is implemented
- THEN the Forge wrapper SHALL preserve existing Forge domain logic and use the runtime service contract for manifest, lifecycle, route, health, and receipt surfaces rather than rewriting Forge internals

#### Scenario: Forge startup emits secret-safe receipts [r[runtime-service-core.forge-slice.receipts]]
- GIVEN the runtime starts or stops the Forge built-in service
- WHEN Forge route registration, gossip enablement, DAG sync worker startup, or health transition occurs
- THEN the runtime SHALL emit receipts that identify the event and service generation without exposing raw keys, cookies, tokens, tickets, or connection strings

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
