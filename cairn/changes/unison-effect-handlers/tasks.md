## Phase 1: Effect manifest model

- [x] [serial] r[molten.effects.manifest_model] Define effect/capability manifest DTOs for executable artifact metadata.
- [x] [serial] r[molten.effects.effect_ids] Assign stable effect ids and canonical schema refs for declared effects.
- [x] [serial] r[molten.effects.artifact_link] Link effect manifests from artifact registry metadata for Wasm, Steel, native, choreography, and job artifacts.
- [x] [parallel] r[molten.effects.no_unison_runtime] Document that Unison abilities are prior art only and that Molten does not implement Unison syntax or generalized algebraic effects.

## Phase 2: Handler binding and admission

- [x] [serial] r[molten.effects.handler_profiles] Define handler profiles for production, local, mock, chaos, profiling, and dry-run execution.
- [x] [serial] r[molten.effects.binding_receipts] Gate handler binding through Basalt/Nickel/Trellis policy and emit Cairn receipts.
- [x] [serial] r[molten.effects.request_envelope] Define canonical effect-request and effect-response envelope shapes with artifact id, effect id, handler profile, input refs, capabilities, and evidence refs.
- [x] [parallel] r[molten.effects.deny_undeclared] Reject effect requests whose effect id is absent from the artifact's admitted manifest.

## Phase 3: First handlers

- [ ] [serial] r[molten.effects.dataspace_handlers] Add local and production handler bindings for dataspace send and observe effects.
- [ ] [serial] r[molten.effects.blob_handlers] Add local and Iroh-backed handler bindings for blob get and blob put effects.
- [ ] [parallel] r[molten.effects.storage_handlers] Add local and Redb-backed handler bindings for typed storage read/write effects.
- [ ] [parallel] r[molten.effects.time_random_handlers] Add deny-by-default clock and random handlers with deterministic local test implementations.
- [x] [parallel] r[molten.effects.wasmtime_hostcall_gate] Check Wasmtime hostcalls against the artifact effect manifest before exposing them.
- [x] [parallel] r[molten.effects.steel_api_gate] Ensure Steel orchestration uses admitted public runtime APIs rather than ambient adapter access.

## Phase 4: Testing and tracing

- [ ] [serial] r[molten.effects.chaos_profile] Add a bounded chaos handler profile for deterministic fault, delay, reorder, and partition injection.
- [ ] [parallel] r[molten.effects.profiling_profile] Add a profiling handler profile that records effect counts, payload sizes, dependency fetches, and trace references.
- [ ] [serial] r[molten.effects.transcript_tests] Add executable transcript tests that pin handler profiles and expected canonical traces/receipts.
- [ ] [parallel] r[molten.effects.property_tests] Add Hegel property tests for deny-by-default behavior, handler substitution, and effect-request determinism.
