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

## Current implementation blocker (2026-07-12)

Evidence-bearing implementation is blocked on three cross-repository inputs that are still active, unimplemented Cairn changes rather than archived receipts:

- Mantle `materialize-spacewasm-reference-cohort`, which owns the exact source commit, offline build closure, rehashable bundle, fixtures, licenses, and consumer handoff;
- Octet `add-spacewasm-mvp-artifact-profile`, which owns the matching static MVP profile and artifact facts;
- ChaosControl `add-spacewasm-mvp-differential-rail`, which owns the matching differential cohort and segmented-execution observations.

No matching SpaceWasm package or archived bundle exists in the sibling repositories. Molten cannot invent those identities, substitute Wasmtime, or label a fixture simulator as SpaceWasm without violating this design's exact-evidence and no-fallback requirements. The existing Wasmtime/component baseline tests pass, but implementation tasks remain intentionally unchecked until the three producer changes archive compatible receipts.
- No silent module rewriting or post-MVP lowering.
