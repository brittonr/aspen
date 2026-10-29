# Shared Wasm Component Runtime Adoption Specification

## ADDED Requirements

### Requirement: Shared runtime prerequisites are complete and immutable

r[molten.wasm_component_adoption.prerequisite] Molten MUST adopt only a completed `hardened-wasmtime-runtime` publication and Mantle consumer-verifier publication pinned to immutable revisions with matching schema and runtime-profile evidence.

#### Scenario: Complete prerequisites are available

- GIVEN both producer changes are archived and immutable revisions expose the required contracts
- WHEN dependency admission runs
- THEN Molten MAY begin dual-run parity under those exact identities.

#### Scenario: A prerequisite is mutable or incomplete

- GIVEN a producer has unchecked tasks, failed gates, mutable source, mismatched schemas, or sibling-only availability
- WHEN dependency admission runs
- THEN cutover MUST remain blocked.

### Requirement: Local and shared paths pass bounded parity

r[molten.wasm_component_adoption.parity] Molten MUST dual-run the existing and shared runtime paths over a frozen positive and negative corpus and MUST compare admission class, world and import checks, normalized outcome, canonical output, resource class, and receipt-payload inputs before cutover.

#### Scenario: Corpus agrees

- GIVEN both paths run the same component, profile, input, authority, effect, and resource facts
- WHEN parity comparison runs
- THEN every selected observation MUST agree under the documented mapping.

#### Scenario: One denial differs

- GIVEN either path admits a rejected fixture or reports a different pre-effect failure class
- WHEN parity comparison runs
- THEN cutover MUST be denied and the local path MUST remain available.

### Requirement: Shared runtime owns product-neutral mechanism only

r[molten.wasm_component_adoption.cutover] After cutover, Wasmtime feature, component-shape, world, import, export, resource, linker, and normalized-outcome mechanism MUST use the shared runtime, while Molten MUST retain profile, actor, extension, effect, replay, materialization, receipt, and policy meaning.

#### Scenario: Product request enters the adapter

- GIVEN Molten admits a component request under its product profile
- WHEN the adapter constructs a shared-runtime request
- THEN it MUST preserve every exact product identity needed for result revalidation
- AND shared runtime types MUST NOT enter pure actor or extension state.

### Requirement: Product hostcalls are explicit and deny ambient WASI

r[molten.wasm_component_adoption.hostcalls] A hostcall-enabled Molten component profile MUST declare a closed product-owned effect-request import over bounded canonical Preserves and MUST bind each operation to current policy, Basalt/UCAN authority, resources, replay, generation, and recorded-effect facts before effect dispatch.

#### Scenario: Declared effect request is admitted

- GIVEN a component imports the exact effect-request function and supplies a declared operation with current matching evidence
- WHEN hostcall admission runs
- THEN the adapter MAY invoke the existing Molten effect port and MUST record the request and response.

#### Scenario: Component requests ambient or undeclared authority

- GIVEN a component imports WASI, filesystem, network, process, environment, clock, random, credential, device, or undeclared product functions
- WHEN linker planning runs
- THEN the import MUST be absent and execution MUST be denied before the observation affects product state.

### Requirement: Ordinary product consumers use the shared runtime

r[molten.wasm_component_adoption.consumers] One ordinary actor and one system extension MUST execute Mantle-materialized components through non-test Molten composition roots using the shared runtime and the same product admission and receipt rails.

#### Scenario: Actor and extension fixtures run

- GIVEN admitted manifests, bundles, profiles, and runtime dependencies
- WHEN the ordinary actor and system-extension entrypoints execute their fixtures
- THEN each MUST produce product-owned execution evidence that binds the shared runtime revision.

#### Scenario: Only direct unit calls exist

- GIVEN shared runtime execution appears only in tests or direct shell calls outside product composition
- WHEN consumer readiness runs
- THEN the adoption MUST remain incomplete.

### Requirement: Molten retains authority and evidence ownership

r[molten.wasm_component_adoption.boundary] Molten MUST retain canonical Preserves, Basalt/UCAN, resource, actor, extension, effect, replay, receipt, admission, and release decisions, and shared runtime success MUST NOT grant or strengthen those decisions.

#### Scenario: Shared runtime reports successful execution

- GIVEN a shared-runtime observation reports success
- WHEN Molten validates the product result
- THEN Molten MUST independently validate output, effect, resource, parent, and receipt facts before product success.

### Requirement: Cutover and rollback are coupled

r[molten.wasm_component_adoption.rollback] The cutover MUST version the shared dependency, adapter, profile, consumer wiring, and fixtures together, and rollback MUST restore the prior dependency and local path together without mixing execution and receipt cohorts.

#### Scenario: Post-cutover regression appears

- GIVEN a parity, runtime, effect, consumer, or receipt regression appears after cutover
- WHEN rollback runs
- THEN Molten MUST restore one coherent prior cohort for both actor and extension consumers.

### Requirement: Adoption has positive and negative validation

r[molten.wasm_component_adoption.validation] Adoption validation MUST cover pure and admitted execution, one admitted and one denied effect request, trap, fuel, deadline, resource denial, malformed output, stale evidence, wrong world, undeclared import, cleanup, parity mismatch, fallback denial, rollback, and non-claims through focused runtime, consumer, Octet, Cairn, and relevant Nix rails.

#### Scenario: Adoption is proposed for archive

- GIVEN implementation, consumer wiring, evidence, and rollback are complete
- WHEN lifecycle validation runs
- THEN all positive and negative cases MUST pass under exact dependency identities.

### Requirement: Adoption evidence preserves non-claims

r[molten.wasm_component_adoption.nonclaims] Passing adoption evidence MUST NOT claim Wasmtime correctness, component correctness, effect correctness, host isolation, actor correctness, extension correctness, whole-system safety, or release eligibility.

#### Scenario: Product fixture is promoted to whole-system proof

- GIVEN a passing actor or extension fixture is labeled as whole-system correctness
- WHEN non-claim validation runs
- THEN the evidence MUST be rejected.
