# Wasm Component Performance Specification

## Purpose

Measure and optimize Mantle-materialized Molten WebAssembly component compilation, instantiation, and execution without weakening deterministic runtime admission or promoting host-scoped measurements into correctness or release claims.

## Requirements

### Requirement: Benchmark suites have exact identity

r[molten.wasm_performance.suite] Molten MUST define benchmark suites with exact Mantle materialization-bundle, component, WIT, runtime-profile, workload-input, phase-marker, host-requirement, measurement, resource-envelope, and sampling identities.

#### Scenario: Suite is unchanged
- GIVEN two runs use identical suite and fixture bytes plus matching declared environment facts
- WHEN suite identity is checked
- THEN both runs MUST resolve to the same BLAKE3 suite identity.

#### Scenario: Workload or profile drifts
- GIVEN materialization bundle, component bytes, WIT, inputs, profile, phase markers, resource bounds, or sampling configuration change
- WHEN suite identity is checked
- THEN the run MUST receive a different identity and MUST NOT be compared as the old suite.

### Requirement: Build-shaped benchmark artifacts come from Mantle

r[molten.wasm_performance.materialization] Portable components, Wizer outputs, and Wasmtime precompiled inputs used by accepted performance suites MUST arrive through verified Mantle materialization bundles; Molten MUST remeasure and admit them but MUST NOT compile production guest components, invoke Wizer, precompile Wasmtime artifacts, or publish replacement build receipts.

#### Scenario: Mantle-materialized variants are benchmarked
- GIVEN portable, transformed, and precompiled variants have complete matching bundles and profile identities
- WHEN a benchmark suite admits them
- THEN Molten MAY measure the declared phases while retaining each distinct artifact and derivation identity.

#### Scenario: Benchmark harness produces its own optimized artifact
- GIVEN the performance rail locally invokes Wizer or Wasmtime precompilation, or receives an optimized artifact without a complete matching Mantle bundle
- WHEN suite admission runs
- THEN Molten MUST reject it from accepted performance evidence.

### Requirement: Performance phases remain separate

r[molten.wasm_performance.phases] Molten MUST record compilation, instantiation, and execution measurements as separate Sightglass phases with engine, target, host, measurement mechanism, and sample metadata.

#### Scenario: Benchmark completes
- GIVEN a supported Wasmtime component benchmark
- WHEN the suite runs
- THEN its receipt MUST preserve separate phase samples and MUST NOT report one aggregate latency as the sole canonical result.

### Requirement: Comparisons require compatible runs

r[molten.wasm_performance.comparison] Molten MUST compare benchmark runs only when suite, component profile, engine cohort, target, host class, measurement mechanism, and resource envelope are compatible, and MUST report deterministic effect-size, confidence, and regression classifications.

#### Scenario: Compatible runs are compared
- GIVEN baseline and candidate runs satisfy every compatibility key and sample requirement
- WHEN comparison runs
- THEN Molten MUST emit a deterministic comparison report over normalized samples.

#### Scenario: Host or runtime differs
- GIVEN baseline and candidate runs differ in host class, runtime, target, measurement mechanism, or component profile
- WHEN comparison is requested
- THEN Molten MUST report them as incompatible and MUST NOT rank one as a regression or improvement.

### Requirement: Precompiled components require exact trusted provenance

r[molten.wasm_performance.aot_admission] Molten MUST deserialize a Wasmtime precompiled component only after exact Mantle and Valence evidence binds the source component, precompiled bytes, Wasmtime cohort, full runtime configuration, WIT profile, target, CPU features, and build inputs.

#### Scenario: Trusted AOT artifact matches
- GIVEN a precompiled artifact and evidence match every admitted identity
- WHEN AOT admission runs
- THEN Molten MAY deserialize it under the matching runtime profile.

#### Scenario: Precompiled bytes are unknown or stale
- GIVEN `.cwasm` bytes are unsigned, tampered, cross-target, cross-profile, or lack exact build/identity evidence
- WHEN AOT admission runs
- THEN Molten MUST deny before unsafe deserialization.

### Requirement: Runtime optimizations are explicit profiles

r[molten.wasm_performance.optimizations] Pooling allocation, copy-on-write heap images, `InstancePre`, compilation strategy, and concurrency/backpressure MUST be explicit bounded runtime-profile facts and MUST rerun deterministic component conformance before producing accepted performance evidence.

#### Scenario: Optimization profile passes conformance
- GIVEN an optimization profile has explicit capacity and resource bounds
- WHEN deterministic conformance and benchmark validation pass
- THEN its performance evidence MAY be retained under that exact profile identity.

#### Scenario: Pool or concurrency capacity is exhausted
- GIVEN a workload exceeds an admitted optimization capacity
- WHEN allocation or scheduling reaches the bound
- THEN Molten MUST apply typed failure or backpressure and MUST NOT silently expand capacity.

### Requirement: Wizer artifacts retain deterministic Mantle build evidence

r[molten.wasm_performance.wizer] A Wizer-preinitialized artifact MUST be Mantle-materialized and MUST bind original bytes, initialization entrypoint, tool identity, denied or deterministically virtualized imports, repeated output identities, transformed bytes, and explicit semantic-equivalence non-claims.

#### Scenario: Repeated Mantle preinitialization matches
- GIVEN identical admitted inputs and deterministic virtual imports produced exact matching outputs in independent Mantle transforms
- WHEN Molten admits the materialization bundle
- THEN the artifact MAY become performance-eligible without Molten invoking Wizer.

#### Scenario: Initialization observed ambient state or lacks build evidence
- GIVEN initialization could observe undeclared clock, randomness, environment, filesystem, network, credentials, or process state, or the transformed bytes lack a complete Mantle receipt
- WHEN Wizer artifact admission runs
- THEN the artifact MUST be rejected or classified as diagnostic-only.

### Requirement: Performance decisions have a functional core

r[molten.wasm_performance.functional_core] Suite and Mantle-bundle validation, compatibility checks, sample normalization, comparison, regression classification, Wizer/AOT manifest admission, and receipt construction MUST be pure deterministic logic over already-loaded facts.

#### Scenario: Identical samples produce identical report
- GIVEN identical normalized suite, environment, and sample facts
- WHEN the comparison core runs
- THEN it MUST return the same report without filesystem, process, clock, network, Wasmtime, or output effects.

### Requirement: Performance evidence preserves bounded roles

r[molten.wasm_performance.evidence] Benchmark and optimization artifacts MUST remain recorded-only performance evidence and MUST state that measurements do not prove correctness, determinism beyond the conformance run, authority, security, cross-machine performance, or release eligibility.

#### Scenario: Benchmark requests proof role
- GIVEN a benchmark artifact requests a correctness/property role or claims general runtime superiority
- WHEN evidence validation runs
- THEN Molten MUST reject the role or overclaim.

### Requirement: Performance rails include positive and negative validation

r[molten.wasm_performance.validation] The performance rail MUST include positive Mantle-materialized baseline/comparison cases and negative incomplete-bundle, local-transform, stale, incompatible, undersampled, exhausted, tampered, nondeterministic-transform, and overclaim cases plus focused lifecycle validation.

#### Scenario: Performance change is reviewed
- GIVEN a suite, runner, optimization, AOT, Wizer, or receipt change
- WHEN validation evidence is assembled
- THEN it MUST include fast focused checks, the applicable deep lane, positive and negative fixtures, deterministic component conformance, and Cairn gates.
