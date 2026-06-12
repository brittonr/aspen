## Phase 1: Central law and deterministic boundaries

- [x] [serial] r[molten.determinism.central_law] Define Molten's deterministic playback law as a central runtime requirement.
- [x] [serial] r[molten.determinism.no_ambient_nondeterminism] Prohibit ambient clock, random, filesystem, network, environment, process, and OS scheduling observations from core/runtime semantics.
- [x] [serial] r[molten.determinism.effect_boundary] Require every external observation to enter through canonical effect request/response envelopes.
- [x] [parallel] r[molten.determinism.identity_inputs] Require artifacts, dependency closure, initial state, schema refs, policy refs, handler profile, seed/log hash, and runtime/tool versions in deterministic run identity.

## Phase 2: Scheduler, logical time, and random

- [x] [serial] r[molten.determinism.scheduler] Define deterministic local actor turn ordering with a total canonical scheduler key.
- [x] [serial] r[molten.determinism.turn_commit] Ensure actors process one event per turn and pending actions become visible only after admitted commit.
- [x] [serial] r[molten.determinism.logical_clock] Implement a logical clock handler for deterministic profiles.
- [x] [parallel] r[molten.determinism.seeded_random] Implement seeded PRNG random handler with request sequencing for deterministic profiles.
- [x] [parallel] r[molten.determinism.chaos_schedule] Define deterministic chaos schedules for faults, delays, drops, reorders, partitions, and resource limits.

## Phase 3: Trace journal and snapshots

- [x] [serial] r[molten.determinism.turn_journal] Emit canonical turn trace records with cause, scheduler key, input hash, before/after state hashes, effect refs, policy refs, committed actions, and receipt refs.
- [x] [serial] r[molten.determinism.snapshot_model] Define canonical snapshot or snapshot-reference model for actor state, dataspace indexes, handler state, logical clock, PRNG state, policy/capability state, and registry/dependency closure.
- [x] [parallel] r[molten.determinism.state_hashes] Compute state hashes from canonical snapshot representations or authenticated snapshot refs.
- [x] [parallel] r[molten.determinism.trace_privacy] Gate access to trace journals and snapshots because they may contain sensitive data or capabilities.

## Phase 4: Handler profiles and replay

- [x] [serial] r[molten.determinism.handler_profiles] Define pure, local, chaos, record, replay, and profiling handler profiles with canonical config hashes.
- [x] [serial] r[molten.determinism.record_profile] Record every effect request/response and relevant external observation under the record profile.
- [x] [serial] r[molten.determinism.replay_profile] Inject recorded effect responses under the replay profile and deny real external side effects.
- [x] [serial] r[molten.determinism.replay_algorithm] Implement replay that verifies input hash, effect requests, committed actions, receipts/traces, outputs, and after state hashes turn by turn.
- [x] [parallel] r[molten.determinism.first_divergence] Report first-divergence diagnostics for scheduler, input, effect request/response, policy decision, action, receipt, trace, output, or state mismatch.

## Phase 5: Integration

- [x] [serial] r[molten.determinism.transcript_integration] Require executable transcripts to pin deterministic identity inputs and compare canonical trace/receipt expectations.
- [x] [serial] r[molten.determinism.eval_cache_integration] Include handler profile, seed/config, initial state hash, policy refs, and dependency closure in evaluation-cache keys.
- [ ] [parallel] r[molten.determinism.remote_sync_integration] Record and replay remote artifact sync discovery, fetch, verification, and admission effects.
- [ ] [parallel] r[molten.determinism.storage_integration] Replay typed storage reads/writes through fixture snapshots or recorded storage effect responses.
- [ ] [parallel] r[molten.determinism.job_dag_integration] Use deterministic local/profiling/chaos profiles and record/replay logs for distributed job DAG testing and incidents.
- [ ] [parallel] r[molten.determinism.upgrade_gate] Allow upgrade sessions to require deterministic transcript or playback success before cutover.

## Phase 6: Tests

- [x] [serial] r[molten.determinism.two_actor_replay_test] Add a local two-actor replay test proving identical artifacts, initial state, profile, and seed produce identical traces and final state hash.
- [x] [serial] r[molten.determinism.random_clock_replay_test] Add tests proving logical clock and seeded random handlers replay deterministically.
- [x] [serial] r[molten.determinism.divergence_tests] Add tests for first-divergence reporting on changed input, effect response, policy decision, and state hash.
- [x] [parallel] r[molten.determinism.no_ambient_tests] Add tests or lints that reject ambient nondeterminism in core/runtime deterministic paths.
- [x] [parallel] r[molten.determinism.property_tests] Add Hegel property tests for replay identity, scheduler total order, trace hash stability, and snapshot authority preservation.
