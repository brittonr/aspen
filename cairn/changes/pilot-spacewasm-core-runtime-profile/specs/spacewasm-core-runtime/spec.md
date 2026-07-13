# SpaceWasm Core Runtime Pilot Specification

## Purpose

Evaluate one exact SpaceWasm cohort as a non-default Molten core-module runtime for the existing canonical Preserves ABI and deterministic segmented actor turns without implying component support, portable state, or production readiness.

## Requirements

### Requirement: SpaceWasm execution uses an explicit pilot profile

r[molten.spacewasm_core.profile] Molten MUST define a typed non-default `spacewasm-core-mvp-pilot` profile that binds the exact engine/materialization/evidence cohort, admitted module features and ABI, resource and segment limits, retention, promotion state, and non-claims.

#### Scenario: Pilot is selected explicitly
- GIVEN an actor manifest names the pilot and every profile identity matches
- WHEN runtime selection runs
- THEN Molten MAY evaluate pilot admission.

#### Scenario: Profile is absent or another runtime is requested
- GIVEN an actor requests Wasmtime core, a component profile, or no explicit engine
- WHEN runtime selection runs
- THEN Molten MUST NOT select SpaceWasm as a fallback.

### Requirement: Evidence-bearing execution requires exact external evidence

r[molten.spacewasm_core.materialization] Molten MUST remeasure a complete Mantle SpaceWasm reference bundle and MUST bind matching Octet static artifact facts and declared ChaosControl differential evidence before classifying pilot execution as evidence-bearing.

#### Scenario: Required evidence matches
- GIVEN bundle members, artifact facts, differential cohort, and module/profile identities match
- WHEN admission runs
- THEN Molten MAY continue to runtime-specific checks and MUST retain all source evidence refs.

#### Scenario: Evidence is missing or stale
- GIVEN a required bundle, artifact report, differential fact, digest, or cohort identity is absent or mismatched
- WHEN admission runs
- THEN Molten MUST deny evidence-bearing execution and MUST NOT fetch or rebuild a fallback.

### Requirement: Module and authority admission precede runtime construction

r[molten.spacewasm_core.admission] Molten MUST validate core-module kind/features, imports/exports, ABI version, artifact identity, resource declarations, authority, policy, and supporting evidence in a pure admission core before constructing a SpaceWasm store or linking host functions.

#### Scenario: Module and grants are admitted
- GIVEN the module uses only admitted features and every hostcall has matching authority, policy, and resource grants
- WHEN admission runs
- THEN the shell MAY receive an execution plan containing only declared bindings.

#### Scenario: Module or grant is invalid
- GIVEN the artifact is a component, uses unsupported proposals, imports WASI/ambient authority, has the wrong ABI, or lacks a required grant
- WHEN admission runs
- THEN Molten MUST deny before runtime construction or side effects.

### Requirement: The pilot preserves the canonical Preserves ABI

r[molten.spacewasm_core.abi] The SpaceWasm shell MUST implement the admitted `molten.wasm.abi.v1` memory/alloc/dealloc and hostcall byte contract with bounded pointer/length validation, canonical Preserves parsing, and exact input/output identities.

#### Scenario: Canonical byte round trip succeeds
- GIVEN bounded canonical actor input and a module exporting the admitted ABI
- WHEN the actor function and declared hostcall complete
- THEN the shell MUST validate canonical output bytes and bind input/output refs in the receipt.

#### Scenario: Memory range or output is invalid
- GIVEN a pointer/length is out of range or returned bytes are malformed, non-canonical, or over limit
- WHEN ABI validation runs
- THEN execution MUST fail closed without accepting the output or releasing unvalidated effects.

### Requirement: Runtime execution is deny-by-default

r[molten.spacewasm_core.execution] The SpaceWasm shell MUST link only admitted `molten:hostcall` functions and MUST NOT provide WASI, filesystem, network, process, environment, clock, random, credential, device, or other ambient host authority.

#### Scenario: Declared hostcall executes
- GIVEN the hostcall name, signature, authority, policy, resource, and input binding match
- WHEN the module invokes it
- THEN the shell MAY dispatch the admitted effect and MUST record the ordered call.

#### Scenario: Undeclared import or hostcall is requested
- GIVEN the module imports or invokes a binding absent from the admitted plan
- WHEN linking or dispatch runs
- THEN the request MUST fail closed and MUST NOT use an ambient fallback.

### Requirement: Interpreter and guest resources are independently bounded

r[molten.spacewasm_core.resources] The pilot profile MUST separately bound interpreter/code/stack allocation, guest linear memory, tables, growth, instruction segments, hostcall bytes, retained continuations, and total turn work and MUST surface allocation or bound failures as typed outcomes without panic-based admission.

#### Scenario: Work stays within all bounds
- GIVEN initialization and execution remain within the admitted resource envelope
- WHEN a segment runs
- THEN resource usage MUST be recorded against the exact profile.

#### Scenario: Interpreter or guest bound is exceeded
- GIVEN allocation, memory, table, growth, stack, instruction, hostcall, continuation, or total-work demand exceeds its bound
- WHEN enforcement runs
- THEN the shell MUST stop or deny with a typed outcome and MUST NOT silently widen another resource class.

### Requirement: Segmented actor turns are generation-fenced and resumable

r[molten.spacewasm_core.resume] Molten MUST distinguish finish, deterministic trap, out-of-fuel yield, admitted host pause, denial, and harness failure and MUST bind every retained continuation to actor generation, module, profile, input, authority, resource, and recorded-effect identities.

#### Scenario: Matching continuation resumes
- GIVEN a retained continuation and all bound facts remain current
- WHEN the scheduler resumes the actor within its total-work bound
- THEN execution MAY continue from the retained in-memory state.

#### Scenario: Continuation facts drift
- GIVEN actor generation, artifact, profile, input, authority, resource, effect log, or retention state differs
- WHEN resume admission runs
- THEN Molten MUST deny and clean up according to the declared policy.

### Requirement: Exact replay compares normalized observations

r[molten.spacewasm_core.receipts] Pilot receipts MUST bind inspection, admission, instantiation, segment, yield/resume, hostcall, resource, terminal, and replay facts, and replay MUST compare terminal class, canonical output, ordered hostcalls, resource class, and selected final-state identity under exact inputs and configuration.

#### Scenario: Segmented replay matches
- GIVEN identical admitted artifacts, inputs, recorded effects, segment plan, and runtime configuration
- WHEN replay completes
- THEN a replay-match receipt MUST bind equal normalized observations.

#### Scenario: Replay differs
- GIVEN any required normalized observation differs
- WHEN replay comparison runs
- THEN Molten MUST emit replay-mismatch evidence and MUST NOT promote the pilot result.

### Requirement: Pilot decisions have a functional core

r[molten.spacewasm_core.functional_core] Profile/evidence/module/ABI/authority/resource/continuation admission, segment planning, observation normalization, replay comparison, receipt DTO construction, and diagnostic ordering MUST be pure deterministic logic.

#### Scenario: Identical facts are evaluated repeatedly
- GIVEN identical normalized inputs
- WHEN the core evaluates them
- THEN it MUST produce identical plans, identities, decisions, and diagnostics without filesystem, network, process, environment, clock, runtime, or output effects.

### Requirement: Pilot evidence preserves non-claims

r[molten.spacewasm_core.nonclaims] Molten MUST keep SpaceWasm pilot evidence distinct from Wasmtime core and component evidence and MUST reject claims of Component Model compatibility, portable state serialization, cross-host migration, SpaceWasm correctness, sandbox completeness, production readiness, or release eligibility.

#### Scenario: Pilot execution passes
- GIVEN all bounded pilot checks and replay fixtures pass
- WHEN operator status is rendered
- THEN the profile MUST remain experimental unless a later accepted change explicitly promotes it.

#### Scenario: Receipt requests a stronger role
- GIVEN a receipt or gate attempts to satisfy component, migration, production, or release requirements with pilot evidence
- WHEN evidence validation runs
- THEN Molten MUST reject the unsupported promotion.

### Requirement: The pilot has positive and negative fixtures

r[molten.spacewasm_core.fixtures] The pilot MUST include positive exact-bundle, admitted-MVP, ABI round-trip, declared-hostcall, expected-trap, resume, replay, and cleanup cases plus negative evidence, feature, component/WASI, import, ABI, pointer/length, canonical-output, resource, continuation, host-trap, replay, fallback, and overclaim cases.

#### Scenario: Pilot behavior changes
- GIVEN profile, admission, ABI, runtime, resource, resume, receipt, or replay behavior changes
- WHEN fixture validation runs
- THEN both positive and negative cases MUST execute under the exact reviewed cohort.

### Requirement: Pilot closeout uses focused validation

r[molten.spacewasm_core.validation] The change MUST run focused pure-core, shell, ABI, resource, resume, replay, executor-conformance, external-evidence, positive/negative fixture, Cairn, and relevant workspace/Nix checks before archive.

#### Scenario: A required check is unavailable
- GIVEN a target, runtime bundle, external evidence rail, or host capability is unavailable
- WHEN closeout evidence is assembled
- THEN it MUST record the exact blocker, affected claim, and next-best deterministic check without marking the unavailable rail passed.
