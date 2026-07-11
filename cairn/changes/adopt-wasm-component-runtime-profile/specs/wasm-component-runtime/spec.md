# Wasm Component Runtime Specification

## Purpose

Execute reviewed WebAssembly components through one deterministic, capability-scoped Molten profile while preserving Preserves as the canonical payload boundary and keeping build, evidence, and authority claims separate.

## Requirements

### Requirement: Versioned component compatibility profile

r[molten.wasm_component.profile] Molten MUST admit WebAssembly components only through a versioned profile that binds the profile schema, Wasmtime, wasm-tools, wit-bindgen, WASI/WIT package versions, enabled Wasm proposals, component world, execution strategy, and deterministic configuration identity.

#### Scenario: Exact cohort is admitted
- GIVEN a component and profile name one supported complete compatibility cohort
- WHEN profile admission runs
- THEN Molten MUST produce one deterministic cohort identity and MAY continue to artifact admission.

#### Scenario: Cohort is stale or partial
- GIVEN a component profile omits a required tool/version/feature fact or mixes unsupported cohort members
- WHEN profile admission runs
- THEN Molten MUST deny before compilation or instantiation with deterministic diagnostics.

### Requirement: WIT is the outer ABI and Preserves remains canonical

r[molten.wasm_component.abi] Molten MUST use a versioned WIT package/world for component entrypoints and hostcalls, while canonical Preserves bytes and schemas MUST remain authoritative for actor payload, hostcall payload, replay, and receipt identity.

#### Scenario: Component implements the admitted world
- GIVEN a component implements the exact admitted world and exchanges a bounded canonical Preserves value
- WHEN generated host and guest bindings invoke the component
- THEN the returned value MUST decode to canonical Preserves bytes and MUST be bound to the WIT world and input identity.

#### Scenario: WIT-compatible output is not canonical Preserves
- GIVEN a component returns bytes accepted by the WIT carrier type but rejected by canonical Preserves decoding
- WHEN output admission runs
- THEN Molten MUST deny the output and MUST NOT emit a successful actor result.

### Requirement: Deterministic Wasmtime execution

r[molten.wasm_component.determinism] Evidence-bearing component execution MUST use deterministic fuel interruption, canonical NaN behavior, deterministic relaxed SIMD or disabled relaxed SIMD, deterministic admitted imports, and a declared memory/table growth strategy that cannot depend on incidental host allocation success.

#### Scenario: Deterministic component is replayed
- GIVEN identical component bytes, profile, WIT, input, recorded effects, policy, authority, resources, and initial state
- WHEN Molten executes and replays the component
- THEN canonical output, hostcall sequence, trap/result class, resource observations, receipts, and final state identity MUST match.

#### Scenario: Component requests nondeterministic growth or host input
- GIVEN a component can observe unbounded memory/table growth, wall clock, host randomness, ambient environment, filesystem, or network state outside the admitted recorded-effect plan
- WHEN deterministic admission runs
- THEN Molten MUST deny before the nondeterministic observation can affect accepted execution evidence.

### Requirement: Component imports do not grant authority

r[molten.wasm_component.authority] Every component import MUST resolve to an explicitly declared host capability plus matching policy, Basalt/UCAN authority, and resource admission; component validity, WIT compatibility, package origin, transport identity, and WASI virtualization MUST NOT grant that authority.

#### Scenario: Declared hostcall is authorized
- GIVEN an imported hostcall has a matching profile declaration, policy decision, current authority record, and resource grant
- WHEN the linker plan is built
- THEN Molten MAY bind only that admitted hostcall implementation.

#### Scenario: Valid component imports undeclared WASI
- GIVEN a syntactically valid component imports a filesystem, socket, clock, random, environment, process, credential, or device interface without complete admission evidence
- WHEN import resolution runs
- THEN Molten MUST deny and MUST leave the interface absent from the runtime linker.

### Requirement: Resource envelopes bound component execution

r[molten.wasm_component.resources] Component admission and execution MUST enforce named bounds for fuel, memory, tables, instances, stack, hostcall bytes, result bytes, and concurrency from the admitted resource profile.

#### Scenario: Component stays inside its envelope
- GIVEN a component's declarations and observed execution remain within every admitted bound
- WHEN execution completes
- THEN the execution receipt MUST record the selected resource profile and bounded observations.

#### Scenario: Component exceeds a bound
- GIVEN compilation, instantiation, a hostcall, or execution would exceed an admitted bound
- WHEN the relevant boundary is reached
- THEN Molten MUST return a typed denial or trap and MUST NOT silently increase the bound.

### Requirement: Component receipts bind identity-changing stages

r[molten.wasm_component.receipts] Molten MUST emit canonical inspection, instantiation, execution, hostcall, denial, and migration evidence that binds exact component BLAKE3, WIT source/package/world identity, compatibility cohort, runtime configuration, imports, policy/authority/resource refs, and Preserves input/output refs.

#### Scenario: Successful execution evidence is checked
- GIVEN an admitted component execution succeeds
- WHEN its receipt is validated
- THEN every stage identity and parent link MUST match the inspected bytes and execution plan.

#### Scenario: Receipt is stale or cross-profile
- GIVEN a receipt refers to different component bytes, WIT, cohort, configuration, import plan, input, or output
- WHEN receipt validation runs
- THEN validation MUST fail closed without treating the receipt as replay or execution evidence.

### Requirement: Core modules and components migrate without fallback

r[molten.wasm_component.migration] Molten MUST classify core modules and components before execution, MUST keep `molten.wasm.abi.v1` separately named during migration, and MUST NOT silently fall back between core-module and component profiles.

#### Scenario: Legacy core module is explicitly requested
- GIVEN an admitted legacy fixture is a core module and requests the compatibility profile
- WHEN execution admission runs
- THEN Molten MAY use the reviewed core-module path and MUST label all evidence with that profile.

#### Scenario: Component execution fails
- GIVEN a component request fails component validation, world matching, linking, or instantiation
- WHEN the failure is handled
- THEN Molten MUST NOT retry the artifact through the legacy core-module ABI.

### Requirement: Runtime decisions have a functional core

r[molten.wasm_component.functional_core] Compatibility validation, WIT/world matching, feature/import/resource admission, deterministic configuration planning, migration classification, and receipt payload construction MUST be pure deterministic logic over already-loaded values.

#### Scenario: Identical facts produce identical plan
- GIVEN identical profile, artifact, WIT, policy, authority, resource, and input facts
- WHEN the runtime core plans execution
- THEN it MUST return the same plan or blockers without filesystem, environment, process, clock, network, Wasmtime, or output effects.

### Requirement: Component evidence preserves non-claims

r[molten.wasm_component.nonclaims] Molten component artifacts and receipts MUST state that WIT compatibility, component validity, deterministic replay, package identity, and successful execution do not prove behavioral correctness, semantic equivalence, authority, release eligibility, or whole-system safety.

#### Scenario: Artifact promotes compatibility to correctness
- GIVEN component evidence claims that matching a world or replaying one fixture proves application correctness
- WHEN evidence validation runs
- THEN Molten MUST reject the overclaim with a deterministic non-claim diagnostic.

### Requirement: Positive and negative conformance evidence

r[molten.wasm_component.fixtures] The component profile MUST include positive execution/replay fixtures and negative malformed, stale, unauthorized, nondeterministic, over-resource, tampered, and fallback fixtures.

#### Scenario: Conformance suite runs
- GIVEN the component profile is proposed for activation or upgrade
- WHEN conformance validation runs
- THEN positive fixtures MUST pass and every negative fixture MUST fail at its declared pre-side-effect boundary.

### Requirement: Component profile validation is reviewable

r[molten.wasm_component.validation] Changes to the component profile MUST run focused core tests, component integration tests, executor conformance, deterministic replay, negative authority/resource tests, Octet checks, and Cairn lifecycle gates.

#### Scenario: Profile change is reviewed
- GIVEN code, WIT, configuration, or receipts for the profile change
- WHEN validation evidence is assembled
- THEN it MUST identify the exact cohort and include both positive and negative results without relying on rendered logs as canonical evidence.
