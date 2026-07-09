# Tasks: runtime-limit-profiles

## Phase 1: Limit model and admission

- [ ] [serial] r[molten.resources.limit_profiles.bounded_selection] Define hard-cap descriptors and runtime limit profile data for representative bounded subsystems.
- [ ] [serial] r[molten.resources.limit_profiles.units_coherence] Add unit/domain labels and coherence checks for related limits.
- [ ] [serial] r[molten.resources.limit_profiles.pure_core] Implement pure limit admission over explicit profile values and override inputs.

## Phase 2: Integration surfaces

- [ ] [parallel] r[molten.resources.limit_profiles.receipt_binding] Bind admitted limits into node service/startup receipts or effective-config readbacks.
- [ ] [parallel] r[molten.resources.limit_profiles.bounded_selection] Thread admitted limits into representative node, live transport, chunk, retention, and harness call sites without raising hard caps.
- [ ] [parallel] r[molten.resources.limit_profiles.units_coherence] Document reviewed limit domains and default-budget caveats.

## Phase 3: Tests and validation

- [ ] [parallel] r[molten.resources.limit_profiles.bounded_selection] Add positive tests for valid limits and negative one-past-hard-cap tests.
- [ ] [parallel] r[molten.resources.limit_profiles.units_coherence] Add negative tests for contradictory timing, frame/session, queue/service, and retention scan relationships.
- [ ] [parallel] r[molten.resources.limit_profiles.pure_core] Add tests proving admission performs no I/O, clock, environment, or service-loop work.
- [ ] [parallel] r[molten.resources.limit_profiles.receipt_binding] Add receipt/readback tests for admitted profile refs and default-budget caveats.
- [ ] [serial] r[molten.resources.limit_profiles.bounded_selection] Run focused resource/node tests and Cairn proposal/design/tasks/spec gates.
