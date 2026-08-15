# Project Specification Delta

## ADDED Requirements

### Requirement: Fabric boundary ownership is explicit

r[molten.modularity.fabric_boundary.inventory] Molten MUST inventory selected fabric contracts, implementations, effects, policy decisions, orchestration, construction sites, and error types.

r[molten.modularity.fabric_boundary.ownership] Each selected item MUST have one owner as pure core logic, application port, application shell, adapter, composition root, or rejected abstraction.

#### Scenario: Inventory classifies one selected contract

r[molten.modularity.fabric_boundary.ownership.classified]
- GIVEN a selected membership, time, entropy, transport, or durable-state contract
- WHEN architecture review runs
- THEN the inventory MUST name its owner, inputs, outputs, failures, effects, and dependency direction.

### Requirement: Applications own fabric ports

r[molten.modularity.fabric_boundary.ports] Genuine external fabric capabilities MUST use narrow application-owned ports with Molten-oriented inputs and outputs.

#### Scenario: Adapter module defines an application port

r[molten.modularity.fabric_boundary.enforcement.adapter_trait]
- GIVEN a maintained adapter module defines a port contract that an application shell consumes
- WHEN architecture validation runs
- THEN validation MUST fail with the contract and owning application scope.

### Requirement: Fabric port errors are typed

r[molten.modularity.fabric_boundary.errors] Maintained fabric ports MUST distinguish domain rejection, capability failure, timeout, cancellation, storage failure, transport failure, and uncertain external outcome where applicable.

#### Scenario: Port returns a raw string failure

r[molten.modularity.fabric_boundary.enforcement.raw_error]
- GIVEN a maintained fabric port returns `String` or another untyped text error
- WHEN architecture validation runs
- THEN validation MUST fail before closeout.

### Requirement: Fabric decisions remain pure

r[molten.modularity.fabric_boundary.core] Profile admission, state transitions, policy decisions, authority-input checks, and uncertainty classification MUST use explicit in-memory inputs without host effects.

#### Scenario: Pure decision returns an effect plan

r[molten.modularity.fabric_boundary.core.plan]
- GIVEN valid state, policy, authority facts, observations, and a request
- WHEN a selected fabric decision runs
- THEN it MUST return deterministic state, events, errors, or a typed effect plan
- AND it MUST NOT execute the planned effect.

### Requirement: Fabric shells own effect order

r[molten.modularity.fabric_boundary.shell] Application shells MUST load facts, call pure decisions, persist required intent, execute only approved effects, and record observed outcomes.

#### Scenario: Authority denial prevents a role effect

r[molten.modularity.fabric_boundary.shell.denial]
- GIVEN assignment authority facts deny a role transition
- WHEN the membership shell handles the command
- THEN it MUST NOT persist an intent, call the role lifecycle port, or report a committed transition.

### Requirement: Fabric adapters contain mechanism code only

r[molten.modularity.fabric_boundary.adapters] Live, simulation, fixture, Iroh, storage, clock, sleep, and entropy adapters MUST implement application ports without owning product policy.

#### Scenario: Live clock supplies an explicit observation

r[molten.modularity.fabric_boundary.adapters.clock]
- GIVEN an admitted live time profile and a selected clock adapter
- WHEN the shell requests a time observation
- THEN the adapter MUST return a typed observation or typed infrastructure failure
- AND the core MUST receive the observation as an explicit value.

#### Scenario: Transport failure remains infrastructure-owned

r[molten.modularity.fabric_boundary.adapters.transport_error]
- GIVEN an Iroh transport attempt fails after submission
- WHEN the adapter reports the result
- THEN the shell MUST preserve an infrastructure or uncertain-outcome classification
- AND it MUST NOT convert that result into a domain policy denial.

### Requirement: Concrete adapters are selected at composition roots

r[molten.modularity.fabric_boundary.composition] Molten MUST select concrete fabric adapters only at reviewed runtime or system-extension composition roots.

#### Scenario: Concrete adapter is selected in the core

r[molten.modularity.fabric_boundary.enforcement.construction]
- GIVEN a declared pure core constructs a live clock, operating-system entropy, Iroh, or persistence adapter
- WHEN dependency-direction validation runs
- THEN validation MUST fail with the construction site.

### Requirement: Fabric migration preserves compatibility

r[molten.modularity.fabric_boundary.compatibility] Boundary migration MUST preserve supported canonical Preserves values, transition refs, receipt meanings, and live or simulation behavior.

r[molten.modularity.fabric_boundary.validation] Tests MUST pair accepted core, shell, and adapter behavior with rejected and malformed behavior.

#### Scenario: Canonical transition and receipt fixtures remain stable

r[molten.modularity.fabric_boundary.compatibility.fixtures]
- GIVEN an accepted pre-migration fabric transition and receipt fixture
- WHEN the migrated path receives equal explicit inputs and observations
- THEN canonical values and refs MUST remain equal unless a separate versioned change approves a difference.

### Requirement: Architecture enforcement remains active

r[molten.modularity.fabric_boundary.enforcement] Maintained checks MUST reject adapter-owned ports, raw string port errors, host effects in core scopes, duplicated policy, and concrete adapter construction outside composition roots.

r[molten.modularity.fabric_boundary.docs] Documentation MUST name core, shell, port, adapter, composition, evidence, and authority ownership.

r[molten.modularity.fabric_boundary.final_checks] Closeout evidence MUST preserve existing transport, durability, timing, entropy, authority, and release non-claims.

#### Scenario: Boundary claims remain scoped

r[molten.modularity.fabric_boundary.final_checks.claims]
- GIVEN source, behavior, compatibility, and lifecycle checks pass
- WHEN Molten states the supported result
- THEN it MUST NOT claim live correctness, global authority, durability, timing accuracy, entropy quality, or release readiness.
