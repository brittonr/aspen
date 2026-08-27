# Molten World Commit Specification Delta

## Purpose

Make every nondeterministic semantic observation explicit, ordered, replayable, and separate from external effect execution.

## ADDED Requirements

### Requirement: Replay profiles inventory every nondeterministic source

r[molten.world_replay_boundary.inventory] Each logical replay profile MUST contain a closed inventory of replay-relevant hostcalls, effect ports, scheduler decisions, asynchronous events, clocks, entropy streams, external reads, and runtime observations. Each source MUST declare exactly one handling class: `deterministic`, `simulated`, `recorded-observation`, or `unsupported`.

#### Scenario: Complete inventory is admitted

- GIVEN every reachable semantic source has one valid handling row
- WHEN profile admission runs
- THEN Molten MUST derive one stable inventory identity

#### Scenario: Unknown hostcall appears

- GIVEN execution reaches a hostcall or effect port absent from the inventory
- WHEN recording or replay evaluates the source
- THEN Molten MUST stop before accepting the transition

### Requirement: Recorded observations bind request and cohort

r[molten.world_replay_boundary.observations] Every recorded observation MUST bind its source, transition step, logical position, operation or request, canonical request identity, result or error class, protection profile, adapter cohort, byte length, and domain-separated BLAKE3 identity.

#### Scenario: External read is recorded

- GIVEN an admitted external-read port returns a bounded value
- WHEN recording seals the observation
- THEN the observation MUST bind the exact request and returned result before successor capture

#### Scenario: Result belongs to another request

- GIVEN a recorded result has the expected source but a different request identity
- WHEN replay admission runs
- THEN Molten MUST reject the observation

### Requirement: Logical ordering is explicit

r[molten.world_replay_boundary.ordering] Scheduler decisions and asynchronous deliveries MUST use explicit replay-stable logical positions. Wall-clock arrival, thread wake order, and adapter iteration order MUST NOT act as hidden ordering authority.

#### Scenario: Events replay in recorded order

- GIVEN two asynchronous events have distinct admitted logical positions
- WHEN replay dispatches them
- THEN it MUST preserve the recorded order before successor comparison

#### Scenario: Events are reordered

- GIVEN the same event values appear in a different logical order
- WHEN observation validation runs
- THEN Molten MUST report the earliest ordering divergence

### Requirement: Completeness is fail-closed

r[molten.world_replay_boundary.completeness] The pure replay core MUST validate inventory closure, expected observation counts, source handling, request bindings, logical order, profile identity, cohort identity, and protection metadata. Missing, duplicate, extra, unsupported, or mismatched observations MUST block replay success.

#### Scenario: One observation is absent

- GIVEN a transition requires a recorded external result and the trace omits it
- WHEN completeness validation runs
- THEN Molten MUST deny replay before transition execution

#### Scenario: Trace contains an extra observation

- GIVEN all declared observations are present and one undeclared extra value remains
- WHEN completeness validation runs
- THEN Molten MUST reject the trace instead of ignoring the value

### Requirement: Replay does not repeat external effects

r[molten.world_replay_boundary.effect_sealing] Replay adapters MUST return sealed recorded observations for external effects. They MUST NOT repeat the original write, send, publish, release, remote call, or other external mutation. Current effect release MUST still pass the existing reservation and promotion protocol.

#### Scenario: Recorded send result replays

- GIVEN a transition contains an admitted recorded send observation
- WHEN logical replay runs
- THEN the adapter MUST return the sealed result without sending again

#### Scenario: Adapter attempts the original effect

- GIVEN a replay adapter tries to execute a sealed external mutation
- WHEN shell effect admission runs
- THEN Molten MUST deny the effect and fail the replay

### Requirement: Trace identity precedes storage optimization

r[molten.world_replay_boundary.trace_identity] Every trace member MUST bind canonical content identity and byte length. Paths, inode identities, hard links, and reflinks MUST NOT substitute for content identity. Sharing optimizations MAY run only after source identity, immutability, policy, and destination readback pass.

#### Scenario: Reflink candidate is valid

- GIVEN source bytes are immutable and match the declared identity
- WHEN the shell creates and reads back a policy-admitted reflink
- THEN it MAY retain the reflink as a storage optimization

#### Scenario: Hard link is presented as identity

- GIVEN a trace member provides only a shared inode or path
- WHEN trace validation runs
- THEN Molten MUST reject the member as unidentified

### Requirement: Native process replay remains opaque and detached

r[molten.world_replay_boundary.opaque_native] Molten MAY attach an exact ChaosControl native-process replay descriptor as an opaque diagnostic member. The descriptor MUST NOT satisfy missing semantic observations, become a logical world root, grant authority, or establish semantic equivalence.

#### Scenario: Opaque diagnostic is compatible

- GIVEN the descriptor and destination match one admitted ChaosControl cohort
- WHEN capsule validation runs
- THEN Molten MAY retain it as detached diagnostic evidence

#### Scenario: Opaque trace replaces a hostcall observation

- GIVEN a logical transition lacks one required hostcall observation but includes a native process trace
- WHEN completeness validation runs
- THEN Molten MUST still reject logical replay

### Requirement: Claims remain bounded

r[molten.world_replay_boundary.claims] Replay receipts MUST state source inventory, handling classes, observation horizon, runtime and adapter cohorts, effect-sealing status, opaque diagnostics, redaction, and non-claims. They MUST NOT claim arbitrary process determinism, kernel replay, external effect completion, host security, authority, or release eligibility.

#### Scenario: Logical replay succeeds

- GIVEN every transition and observation matches for one exact profile
- WHEN the receipt is emitted
- THEN it MUST limit success to that profile, trace, horizon, and cohorts

### Requirement: Verification covers success and denial paths

r[molten.world_replay_boundary.verification] Verification MUST cover deterministic, simulated, recorded, asynchronous, sealed-effect, stable-replay, and opaque-diagnostic success paths. It MUST also cover unknown, missing, mismatched, reordered, duplicate, extra, unsupported, drifted, secret-bearing, effect-repeating, tampered, and overclaim denial paths.

#### Scenario: Negative corpus is incomplete

- GIVEN fixtures omit unknown-source, missing-result, wrong-order, repeated-effect, tamper, or overclaim cases
- WHEN verification coverage is evaluated
- THEN the change MUST remain incomplete
