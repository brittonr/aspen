## Context

The current reviewed Wasm actor path validates core modules with `wasmparser`, rejects ambient/WASI imports, configures Wasmtime fuel and store limits, links only declared `molten:hostcall` functions, and exchanges canonical Preserves bytes through `molten.wasm.abi.v1`. This is a strong fail-closed shell, but its manual memory ABI duplicates work owned by the Component Model canonical ABI and does not give system extensions or external adapters one typed compatibility target.

The new profile must improve interoperability without transferring canonical value identity from Preserves, authority from Basalt/UCAN, evidence semantics from Valence, or build trust from Mantle into Wasmtime or WIT.

## Decisions

### 1. WIT owns the outer ABI; Preserves owns canonical payload semantics

**Choice:** Define a versioned WIT package/world for component entrypoints, hostcalls, and explicit error results. Domain payloads cross that boundary as bounded canonical Preserves byte carriers or narrowly typed control fields. The WIT package, world, and source bytes receive independent BLAKE3 identities.

**Rationale:** The Component Model removes manual pointer plumbing and permits generated host/guest bindings, while Preserves continues to provide the stack's canonical value, replay, and receipt identity.

### 2. One compatibility cohort is admitted at a time

**Choice:** A typed Nickel profile declares the exact Wasmtime, wasm-tools, wit-bindgen, WASI/WIT package versions, enabled Wasm proposals, component world, runtime strategy, and profile schema. Rust consumes a checked deterministic export. Unknown, partially specified, or independently upgraded cohorts deny before compilation or instantiation.

**Rationale:** Component tooling evolves as a cohort. Independent version drift can compile successfully while changing generated bindings, canonical ABI lowering, imports, or runtime behavior.

### 3. Determinism is an explicit execution plan

**Choice:** A pure core derives an execution plan from the admitted profile and inspected artifact facts. Evidence-bearing execution uses deterministic fuel interruption, NaN canonicalization, deterministic relaxed SIMD or disables relaxed SIMD, rejects nondeterministic memory/table growth unless the profile proves an up-front fixed allocation strategy, and receives clocks, randomness, environment, files, and network results only through recorded admitted effects.

**Rationale:** WebAssembly is mostly deterministic, not automatically deterministic. Runtime defaults and host imports must not silently define replay semantics.

### 4. Component validity and capability authority remain separate gates

**Choice:** Component inspection verifies syntax, world conformance, imports, exports, feature posture, and resource declarations. A separate admission step resolves every import to a declared host capability, policy decision, Basalt/UCAN authority record, and resource grant. The Wasmtime component linker is populated only from the admitted plan.

Build-time WASI-Virt composition may reduce imports, but the runtime linker still denies undeclared imports.

**Rationale:** A valid or virtualized component has no inherent right to filesystem, network, clock, random, environment, process, credential, or device access.

### 5. Resource limits are profile facts, not ambient defaults

**Choice:** The profile names bounded fuel, linear-memory, table, instance, stack, hostcall-byte, result-byte, and concurrency limits through named policy fields. The core validates artifact declarations against those bounds; the shell configures Wasmtime and records observed usage or typed denial.

**Rationale:** Component canonical ABI machinery does not remove denial-of-service risks or resource-accounting obligations.

### 6. Receipts bind every identity-changing stage

**Choice:** Inspection, instantiation, execution, and hostcall receipts bind exact component BLAKE3, WIT source/package/world identity, compatibility cohort, Wasmtime configuration identity, imported capability set, policy/authority/resource refs, Preserves input/output refs, fuel/resource observations, and result or trap class.

**Rationale:** A component filename or WIT world alone cannot identify what executed or which authority was exercised.

### 7. Core modules and components are distinct migration profiles

**Choice:** Keep `molten.wasm.abi.v1` as an explicitly requested compatibility profile while adding the component profile. Artifact inspection classifies the binary before execution. A component request never falls back to the core-module ABI, and a core module is never relabeled as a component merely because WIT metadata is embedded.

**Rationale:** Silent fallback would weaken type, import, and receipt expectations and make replay evidence ambiguous.

### 8. System-extension execution consumes this profile

**Choice:** The active `system-extension-service-runtime` change may select the component profile as one separately admitted execution profile, but it continues to own callback lifecycle, generation fencing, fabric ports, supervision, and service semantics.

**Rationale:** Component execution is a mechanism. It must not absorb the system-extension lifecycle or fabric authority model.

### 9. Mantle materialization is the production artifact boundary

**Choice:** Evidence-bearing and production execution consumes a versioned Mantle materialization bundle binding exact portable or precompiled bytes, WIT/package inputs, build cohort, expected runtime profile, stage receipts, and the Octet report produced before materialization. Policy-required Valence sidecars and Cairn acceptance receipts arrive in a separate admission envelope keyed to the canonical bundle and child identities so no evidence hash is circular. Molten remeasures bytes, re-inspects runtime-relevant facts, and performs independent capability/resource admission; it never treats a store path or Mantle success as runtime authority. Direct loose-byte loading remains available only to an explicitly test-only profile that cannot emit production evidence.

**Rationale:** Build, composition, virtualization, Wizer, and precompilation are reproducible materialization concerns. Keeping them out of the runtime prevents toolchain drift and makes unsafe `.cwasm` deserialization depend on one exact authenticated derivation chain.

## Functional core / imperative shell split

- **Pure core**: compatibility-cohort validation, Mantle bundle/identity admission, artifact-fact admission, WIT/world matching, import/capability resolution, deterministic configuration planning, resource decisions, migration classification, and receipt payload construction over already-loaded values.
- **Imperative shell**: read and rehash materialized artifact/WIT bytes, invoke wasm-tools parsers, create Wasmtime engines/linkers/stores, instantiate and call components, perform admitted host effects, and persist/render receipts; it does not build, compose, virtualize, transform, or precompile production components.

## Risks / Trade-offs

- Component tooling is still evolving. Pinning a cohort reduces drift but requires deliberate upgrade changes and conformance reruns.
- Mantle bundle and Molten runtime profiles can advance at different rates. Unknown or incompatible profile identities deny rather than triggering a local rebuild or fallback.
- Canonical ABI lowering adds runtime machinery and may change performance. Performance optimization remains a separate evidence package after deterministic conformance passes.
- A byte-carrier WIT ABI is less domain-typed than duplicating Preserves schemas in WIT, but it avoids competing canonical schemas and permits incremental typed control fields.
- WASI-Virt library defaults can pass host subsystems through. Any direct library integration must construct explicit deny-all state before applying reviewed grants.

## Non-Goals

- No replacement of Preserves with WIT values as canonical Molten data.
- No authority derived from component validity, imports, signatures, package origin, or transport identity.
- No automatic adoption of WASIp3, wRPC, OpenTelemetry-WASI, JavaScript, Python, WAMR, or another runtime.
- No production component compilation, composition, WASI virtualization, Wizer transformation, or Wasmtime precompilation in Molten.
- No claim that matching WIT worlds, replay outputs, or component hashes prove behavioral correctness or semantic equivalence.
