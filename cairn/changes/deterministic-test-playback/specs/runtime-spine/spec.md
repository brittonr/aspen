## ADDED Requirements

### Requirement: Deterministic playback law
r[molten.determinism.central_law] Molten MUST define deterministic playback as a central runtime law: the same artifacts, dependency closure, initial state, schema refs, policy refs, handler profile, and seed or recorded effect log produce the same canonical traces, receipts, outputs, and final state hash.

#### Scenario: Same replay identity reproduces refs
- GIVEN a deterministic runtime run with fixed artifacts, initial state, handler profile, and seed or recorded effect log
- WHEN the run is replayed with the same identity inputs
- THEN the replay emits matching canonical trace refs, receipt refs, outputs, and final state hash

### Requirement: No ambient nondeterminism in deterministic profiles
r[molten.determinism.no_ambient_nondeterminism] Deterministic runtime profiles MUST NOT observe ambient clock, random, filesystem, network, environment, process, or OS scheduling inputs except through explicit recorded or deterministic effect responses.

#### Scenario: Ambient observation is denied
- GIVEN a deterministic or replay handler profile
- WHEN runtime execution attempts to observe an external source without an admitted effect response
- THEN execution is denied before semantic state changes and emits deterministic diagnostics

### Requirement: External observations cross effect boundaries
r[molten.determinism.effect_boundary] Runtime observations of clock, random, storage, blobs, network, process, filesystem, policy, and external services MUST cross canonical effect request and effect response boundaries before affecting deterministic state.

#### Scenario: Effect response enters replay identity
- GIVEN a runtime turn that needs a clock or random observation
- WHEN an admitted handler returns the observation
- THEN the canonical effect request and response refs are included in trace, receipt, and replay identity evidence

### Requirement: Replay identity binds all deterministic inputs
r[molten.determinism.identity_inputs] Deterministic replay identity MUST bind artifacts, dependency closure, initial state, schema refs, policy refs, capability state, handler profile, seed or effect-log hash, and runtime/tool versions where those inputs affect execution.

#### Scenario: Changed identity input diverges
- GIVEN a recorded deterministic run
- WHEN replay changes an input bound into replay identity
- THEN replay fails at the first changed boundary and reports expected and actual canonical refs

### Requirement: Total deterministic scheduler key
r[molten.determinism.scheduler] Local deterministic runtime scheduling MUST use a documented total canonical key and MUST NOT depend on map iteration order, thread races, or live arrival timing after events are admitted.

#### Scenario: Queue order is stable
- GIVEN two admitted events with canonical scheduler keys
- WHEN a deterministic profile selects the next actor turn
- THEN the event with the smaller canonical scheduler key is selected regardless of host scheduling behavior

### Requirement: Turn commit visibility
r[molten.determinism.turn_commit] Actors MUST process one event per turn and pending state, assertion, message, effect-intent, and evidence changes MUST become visible only after admitted commit.

#### Scenario: Denied turn rolls back pending changes
- GIVEN a turn with staged mutations and pending outbound actions
- WHEN admission or execution denies the turn
- THEN pending changes are discarded and the trace records a rollback or denial receipt

### Requirement: Logical clock handler
r[molten.determinism.logical_clock] Deterministic profiles MUST provide logical clock observations through explicit handler responses rather than ambient wall-clock reads.

#### Scenario: Logical time replays
- GIVEN a recorded logical clock effect response
- WHEN replay reaches the same clock request
- THEN replay injects the recorded logical time and denies any ambient wall-clock read

### Requirement: Seeded random handler
r[molten.determinism.seeded_random] Deterministic profiles MUST provide random bytes from explicit seed/config and request sequence or from recorded responses.

#### Scenario: Seeded random replays
- GIVEN a deterministic seed and random request sequence
- WHEN the same run is replayed
- THEN the random response refs match the recorded response refs

### Requirement: Deterministic chaos schedule
r[molten.determinism.chaos_schedule] Chaos profiles SHOULD represent faults, delays, drops, reorders, partitions, and resource limits as deterministic schedules bound into replay identity.

#### Scenario: Chaos fault is replayable
- GIVEN a chaos profile with a seeded fault schedule
- WHEN a delivery is delayed or dropped
- THEN the trace records the schedule position and replay reproduces the same fault decision

### Requirement: Turn journal evidence
r[molten.determinism.turn_journal] Deterministic runtime turns MUST emit canonical turn journal records with cause, scheduler key, input hash, before/after state hashes, effect refs, policy refs, committed actions, and receipt refs sufficient for replay comparison.

#### Scenario: Journal binds turn state
- GIVEN a committed deterministic actor turn
- WHEN the turn journal is emitted
- THEN it binds input, effect, policy, action, receipt, before-state, and after-state refs

### Requirement: Snapshot model
r[molten.determinism.snapshot_model] Replay MUST start from a canonical snapshot or authenticated snapshot refs covering runtime state, handler state, policy/capability state, dependency closure, and relevant storage or fixture refs.

#### Scenario: Snapshot seeds replay
- GIVEN a replay run with a snapshot ref
- WHEN replay initializes runtime state
- THEN state is derived from that snapshot and no additional authority is minted

### Requirement: State hashes
r[molten.determinism.state_hashes] Deterministic runtime profiles SHOULD compute state hashes from canonical snapshot representations or authenticated snapshot refs.

#### Scenario: State hash mismatch stops replay
- GIVEN a recorded after-state hash for a turn
- WHEN replay computes a different after-state hash
- THEN replay stops and reports a state-hash divergence

### Requirement: Trace privacy gates
r[molten.determinism.trace_privacy] Trace journals and snapshots that may contain secrets or capabilities MUST be subject to policy admission before export or rendering.

#### Scenario: Sensitive trace export is denied
- GIVEN a trace containing secret or capability-bearing refs
- WHEN an unauthorized export is requested
- THEN export denies or emits a redacted view without revealing protected content

### Requirement: Handler profiles
r[molten.determinism.handler_profiles] Molten MUST define pure, local, chaos, record, replay, and profiling handler profiles with canonical profile ids and config hashes.

#### Scenario: Profile id is evidence
- GIVEN a runtime report or receipt from a deterministic run
- WHEN the report is inspected
- THEN it includes the handler profile identity and enough config evidence to distinguish profile behavior

### Requirement: Record profile
r[molten.determinism.record_profile] Record profiles MUST record every admitted external effect response and relevant observation needed for later replay.

#### Scenario: Production observation is recorded
- GIVEN a record-profile run that calls a real adapter
- WHEN the adapter returns an observation
- THEN the canonical response evidence is stored in the effect log before affecting deterministic state

### Requirement: Replay profile
r[molten.determinism.replay_profile] Replay profiles MUST inject recorded effect responses, compare effect requests for exact match, and deny real external side effects.

#### Scenario: Replay does not consult outside world
- GIVEN a recorded effect log
- WHEN replay reaches an effect request
- THEN it compares the request ref, injects the recorded response, and does not call live external adapters

### Requirement: Replay algorithm
r[molten.determinism.replay_algorithm] Replay MUST verify input hashes, effect requests, effect responses, committed actions, receipts or traces, outputs, and after-state hashes turn by turn.

#### Scenario: Replay compares turn boundaries
- GIVEN a recorded deterministic run
- WHEN replay processes each turn
- THEN it checks the canonical refs at each semantic boundary before continuing to the next turn

### Requirement: First divergence diagnostics
r[molten.determinism.first_divergence] Replay SHOULD report the first divergent boundary with divergence kind, expected and actual canonical refs, handler profile, seed or log position, actor or turn id, and safe diagnostics.

#### Scenario: Input divergence is first
- GIVEN a recorded run and a replay with a changed input
- WHEN replay compares the turn input ref
- THEN replay stops at the input boundary and reports expected and actual input refs

### Requirement: Transcript integration
r[molten.determinism.transcript_integration] Executable transcripts MUST pin deterministic identity inputs and compare canonical trace, receipt, output, or diagnostic expectations.

#### Scenario: Transcript pins replay identity
- GIVEN an executable transcript for a deterministic runtime scenario
- WHEN the transcript is run as evidence
- THEN the report binds initial state, handler profile, seed or log hash, policy refs, and expected canonical outputs

### Requirement: Evaluation cache integration
r[molten.determinism.eval_cache_integration] Evaluation cache keys MUST include handler profile, seed/config, initial state hash, dependency closure, policy refs, and other deterministic identity inputs that affect results.

#### Scenario: Cache rejects changed profile
- GIVEN a cached deterministic result for one handler profile
- WHEN the same artifact runs under a different profile
- THEN the cache key differs or the entry is rejected

### Requirement: Remote sync replay integration
r[molten.determinism.remote_sync_integration] Remote artifact sync SHOULD record discovery, fetch, verification, and admission effects so replay can validate remote execution setup without live network dependence.

#### Scenario: Remote fetch is replayed from records
- GIVEN a recorded remote artifact fetch
- WHEN replay validates setup
- THEN it uses recorded fetch and verification evidence rather than live peer timing

### Requirement: Storage replay integration
r[molten.determinism.storage_integration] Typed storage replay SHOULD use fixture snapshots or recorded storage effect responses for deterministic reads and writes.

#### Scenario: Storage read is recorded
- GIVEN a production storage read in a record profile
- WHEN replay reaches the same read request
- THEN replay injects the recorded storage response and compares the request ref

### Requirement: Job DAG replay integration
r[molten.determinism.job_dag_integration] Distributed job DAG tests SHOULD use deterministic local, profiling, or chaos profiles and production incidents SHOULD be replayable from recorded effect logs where possible.

#### Scenario: Job replay binds handler profile
- GIVEN a job receipt with handler profile and effect-log refs
- WHEN replay validates the job
- THEN it checks profile identity and recorded effect refs before accepting matching output refs

### Requirement: Upgrade replay gate
r[molten.determinism.upgrade_gate] Upgrade sessions MAY require deterministic transcript or recorded playback success before cutover.

#### Scenario: Upgrade blocks on replay failure
- GIVEN an upgrade session with a required replay gate
- WHEN replay reports a divergence
- THEN the upgrade cutover is denied before mutation

### Requirement: Two-actor replay test
r[molten.determinism.two_actor_replay_test] Molten SHOULD include a local two-actor or two-object replay test proving identical artifacts, initial state, profile, and seed produce identical traces and final state hash.

#### Scenario: Vat replay fixture is stable
- GIVEN the local vat replay fixture with a fixed seed, profile, and initial object state
- WHEN the same two-object run is replayed
- THEN the fixture emits matching trace and final-state refs

### Requirement: Random and clock replay tests
r[molten.determinism.random_clock_replay_test] Molten SHOULD test that logical clock and seeded random handlers replay deterministically.

#### Scenario: Recorded clock response is stable
- GIVEN a deterministic clock or random response in the effect log
- WHEN replay runs with the same request sequence
- THEN the response refs match the recorded refs

### Requirement: Divergence tests
r[molten.determinism.divergence_tests] Molten SHOULD test first-divergence reporting for changed input, effect response, policy decision, and state hash boundaries.

#### Scenario: Changed response reports effect divergence
- GIVEN a recorded deterministic run
- WHEN replay uses a changed effect response
- THEN replay reports an effect-response divergence before comparing downstream state

### Requirement: No ambient tests
r[molten.determinism.no_ambient_tests] Molten SHOULD include tests or lints that reject ambient nondeterminism in deterministic core or runtime paths.

#### Scenario: Direct ambient read is rejected
- GIVEN code marked as deterministic runtime core
- WHEN it attempts to use ambient clock, random, filesystem, network, environment, process, or scheduler observations
- THEN tests or gates reject the change or require an explicit effect boundary

### Requirement: Determinism property tests
r[molten.determinism.property_tests] Molten SHOULD include property tests for replay identity, scheduler total order, trace hash stability, and snapshot authority preservation.

#### Scenario: Generated replay identity is stable
- GIVEN generated deterministic inputs within bounded limits
- WHEN the same identity is replayed
- THEN canonical trace and final-state refs remain stable
