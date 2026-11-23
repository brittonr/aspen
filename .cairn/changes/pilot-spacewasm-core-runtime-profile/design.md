## Context

Molten's current harness validates core-module bytes with wasmparser, permits only declared `molten:hostcall` functions, denies WASI, enforces fuel and store limits, and optionally exchanges canonical Preserves bytes through exported memory/alloc/dealloc functions. SpaceWasm supports compatible primitive host signatures and linear-memory access in principle, but its current feature set is WebAssembly 1.0 only and its interpreter resource allocator is separate from guest linear memory.

The pilot must answer two bounded questions: whether the existing Molten core ABI executes correctly under one exact SpaceWasm cohort, and whether segmented SpaceWasm execution can preserve Molten's deterministic actor observations. It must not reopen the Component Model decision or make NASA provenance a trust root.

## Decisions

### 1. Runtime selection is explicit and non-default

The executor profile will include a closed engine kind for the SpaceWasm pilot and exact cohort/profile identities. A core Wasmtime request, component request, or absent profile cannot select SpaceWasm implicitly. Production and release gates remain false unless a later accepted change promotes the profile.

### 2. Admission uses exact materialized and independent evidence

Evidence-bearing pilot execution requires a complete remeasured Mantle bundle, matching Octet static artifact/profile facts, and a matching ChaosControl differential cohort/result class when that rail is available. Missing evidence produces a typed blocker; test-only loose artifacts remain clearly diagnostic.

### 3. A pure core decides compatibility before runtime construction

The admission core will validate engine/profile identity, module kind/features, imports/exports, ABI version, artifact BLAKE3, memory/table declarations, growth policy, interpreter/linear-memory/stack/instruction limits, authority/resource decisions, and evidence bindings from explicit DTOs.

### 4. The shell reuses canonical Preserves semantics

The SpaceWasm shell will implement the existing `molten.wasm.abi.v1` pointer/length/result contract and declared `molten:hostcall` functions. It will copy bounded canonical bytes, validate pointer/length ranges, reparse outputs, and retain canonical input/output refs. No WASI, filesystem, network, environment, clock, random, process, credential, or device imports are linked.

### 5. Guest and interpreter memory limits are independent

The profile will separately cap SpaceWasm interpreter/code/stack allocation and guest linear memory. Fixed-page interpreter allocation is not evidence that guest memory is bounded. `memory.grow` is denied unless the exact profile explicitly admits a bounded deterministic policy.

### 6. Actor turns may yield only at declared boundaries

The shell will run a declared instruction segment and return finish, trap, out-of-fuel yield, host pause, or harness failure. A continuation remains generation-, actor-, artifact-, profile-, input-, authority-, resource-, and effect-log-bound. Resume under stale or mismatched facts is denied.

### 7. Replay compares normalized observations

For exact artifacts, input, recorded effects, segment plan, and configuration, replay will compare terminal class, canonical output, ordered hostcalls, resource class, and selected final state identity. Raw instruction counters remain runtime facts. An uninterrupted-versus-segmented match is required for promoted fixtures.

### 8. Interpreter state is not a portable artifact

SpaceWasm exposes resumable in-memory state, but this change will not serialize pointers, allocator state, stores, tables, or host closures. Checkpoint migration and cross-host continuation remain explicit non-claims until a canonical state contract and independent validation exist.

### 9. Receipts retain engine-specific facts and non-claims

Inspection, admission, instantiation, segment, yield/resume, hostcall, terminal, resource, and replay receipts will bind source commit/build, IR/support profile, allocator configuration, module/profile/input/effect identities, instruction plan, and observations. They remain separate from Wasmtime/component receipts.

## Functional core / imperative shell split

- **Pure core**: profile/evidence/module/ABI/resource/authority admission, continuation-binding validation, segment planning, outcome normalization, replay comparison, receipt DTO construction, and deterministic diagnostics.
- **Imperative shell**: bundle reads/remeasurement, SpaceWasm store/module setup, streaming decode, memory transfer, host function linkage, segment execution, effect dispatch after admission, state retention, and receipt persistence/rendering.

## Risks / Trade-offs

- Rust-produced `wasm32-unknown-unknown` modules may require unsupported proposals. The pilot must use a constrained producer profile or reject them; no opaque lowering is allowed.
- SpaceWasm is slower and less mature than Wasmtime. Performance evidence remains separate and cannot weaken deterministic or authority gates.
- In-memory continuations increase lifecycle complexity. Generation fencing, finite retention, and explicit cleanup are required.
- Unsafe upstream containers remain in the runtime trust boundary even when differential tests pass.
- Matching one ABI fixture does not generalize to arbitrary actors or platforms.

## Non-Goals

- No replacement of Wasmtime or the planned Component Model runtime.
- No WIT, components, WASI, plugin-default, or ambient capability support.
- No state serialization, migration, live upgrade, or cross-host resume claim.
- No flight qualification, runtime correctness, sandbox-completeness, or release-readiness claim.

## Evidence-bearing prerequisites

The source-side pilot and its bounded negative controls can be checked locally. Evidence-bearing execution still requires a complete, independently admitted producer cohort:

- Mantle's `materialize-molten-spacewasm-abi` locally verified bundle `774331f6c2defe75e30c354b2b3080838cf56438bbd99268779fe4590f8c9f39` on 2026-10-30, but its immutable-revision/bundle publication task remains unchecked;
- Octet archived the matching static `spacewasm-molten-abi-pilot` profile, but its fixture receipt is not a receipt for the exact selected Mantle members or an independently selected immutable verifier executable;
- ChaosControl [`a2b85741`](https://github.com/brittonr/chaoscontrol/commit/a2b85741c341863798850fa3c82d19a22e4912b3) implements the opt-in full-ABI hostcall model and Wasmtime 49.0.5 reference runner. Its synthetic fixtures do not run the SpaceWasm side or emit the required `chaoscontrol.spacewasm-molten-abi-differential.v1` report for the exact published members.

The expected bundle/cohort/profile/probe identities and Nix-store verifier executable hashes must be selected through their actual publication/admission boundary, never copied from a candidate's self-consistent reports. Do not substitute hostless receipts, the Wasmtime reference run alone, local fixture grants or silent module lowering for the missing two-engine report. Local checks do not close these evidence-bearing acceptance tasks.
