## Context

Molten combines reactive dataspaces, optional vats, choreography, remote artifact sync, typed durable storage, effect handlers, transcripts, and policy/evidence gates. These pieces need a common determinism contract. If a transcript passes once but cannot be replayed, or if a receipt cannot be tied to reproducible trace data, the runtime loses much of its audit value.

Determinism does not mean every production run is free of external effects. It means all external observations enter through explicit effect responses that can be recorded, hashed, and replayed. Production execution can record nondeterministic reality; replay execution injects the recorded observations and verifies the same internal behavior follows.

## Central Law

Molten MUST preserve this law for deterministic profiles and recorded playback:

```text
same artifacts
+ same dependency closure
+ same initial state
+ same schema refs
+ same policy refs and capability state
+ same handler profile
+ same deterministic seed or recorded effect log
= same canonical trace records, receipts, outputs, and final state hash
```

If the law fails, Molten MUST stop replay at the first divergent boundary and report enough canonical evidence to diagnose the difference.

## Goals

- Make deterministic test/playback a required runtime property for local deterministic profiles and recorded replays.
- Remove ambient nondeterminism from core and adapter-facing runtime code.
- Route clock, random, network, storage, blob, remote execution, process, filesystem, policy, and external service observations through effect handlers.
- Define deterministic actor turn ordering and replayable turn journals.
- Provide record and replay handler profiles for production-like executions.
- Provide first-divergence diagnostics based on canonical hashes and trace records.
- Make executable transcripts and evaluation cache keys include all determinism inputs.

## Non-Goals

- Do not claim that two live production runs with real network timing and real clocks will be bit-identical unless those effects are recorded and replayed.
- Do not require deterministic OS thread scheduling; Molten must avoid depending on it for semantic ordering.
- Do not make wall-clock timestamps, random bytes, filesystem listings, environment variables, or network delivery ambient inputs.
- Do not let replay handlers perform external side effects.
- Do not treat a matching trace as authorization; policy and capability checks are still required.

## Determinism boundary

Core validation is pure. Runtime code may observe the outside world only via admitted effect requests. Effect requests and responses are canonical Preserves values with:

- actor/execution id,
- artifact id,
- effect id,
- sequence number,
- handler profile id,
- canonical input hash or content ref,
- policy/capability/evidence refs,
- canonical response hash or denial record.

A deterministic profile must produce responses from pinned state, pinned seed/config, or recorded effect logs. A production profile may call real adapters but must record enough response data to replay later.

## Scheduler law

Local scheduling must not depend on OS thread race, map iteration order, or arrival timing once events are admitted to the deterministic queue. Actor turns are ordered by a canonical key such as:

```text
logical_time, priority, queue_sequence, target_actor_id, sender_id, envelope_hash
```

The exact key may evolve, but it must be total, documented, canonical, and included in trace records. Actors process one event per turn. Pending actions become visible only at commit. If a turn fails or is denied, pending actions are discarded and the denial is traced.

## Logical time and randomness

Clock and random are effects:

- `Clock` under deterministic profiles returns logical time from the scheduler or a scripted timeline.
- `Random` under deterministic profiles uses an explicit seeded PRNG with recorded request sequence.
- Production wall-clock and entropy sources are allowed only through admitted handlers and must record observed values for replay.

Trace records distinguish logical time, wall-clock observations, and replay-injected time.

## Handler profiles

Required profiles:

- `pure`: no effects admitted.
- `local`: deterministic in-process dataspace, blob, storage, clock, and random handlers.
- `chaos`: deterministic local/profile execution with seeded failures, delays, drops, reorders, partitions, and resource limits.
- `record`: admitted real adapters may run, but every effect response and relevant external observation is recorded canonically.
- `replay`: no real external effects; recorded responses are injected and requests are checked for exact match.
- `profiling`: deterministic profile with cost, allocation, network-estimate, hot-spot, and trace metadata.

A handler profile id and its config hash are part of transcript, job, cache, and playback identity.

## Turn journal

Every committed turn and significant adapter event should emit a canonical trace record containing:

- turn id,
- parent/cause turn id,
- actor/session/vat ids,
- triggering event/envelope hash,
- scheduler key,
- before state hash,
- effect requests and response refs,
- policy decisions and receipt refs,
- pending actions,
- committed assertions/retractions/messages/effects,
- emitted envelopes/content refs,
- after state hash,
- error/denial info if applicable.

The journal is data, not just logs. It can be stored, hashed, filtered, rendered, attached to receipts, and used by replay.

## Snapshots and state hashes

Replay starts from a known initial state. A state snapshot should include or reference:

- actor states and live assertions,
- dataspace indexes,
- vat actormap snapshots where applicable,
- protocol endpoint states,
- typed storage refs or fixture store snapshot,
- handler profile state,
- logical clock and deterministic PRNG state,
- capability/revocation/policy state,
- artifact registry snapshot or dependency closure hash.

State hashes are over canonical snapshot representations or authenticated references. Snapshots must not mint authority that was not present in the recorded state.

## Replay algorithm

Replay should:

1. Load the initial snapshot and dependency closure.
2. Install the replay handler profile and recorded effect log.
3. For each recorded turn, deliver the expected event by canonical queue order.
4. Verify the input/event hash.
5. Run the turn until each effect request.
6. Compare the effect request to the recorded request.
7. Inject the recorded response.
8. Compare pending and committed actions.
9. Compare emitted receipts and trace records where deterministic.
10. Compare the after state hash.

On mismatch, replay stops and reports first-divergence diagnostics.

## First-divergence diagnostics

Diagnostics should include:

- divergence kind: scheduler, input, effect request, effect response, policy decision, action, receipt, trace, output, or state hash.
- expected and actual canonical hashes,
- minimal rendered diff where safe,
- artifact ids and dependency closure hash,
- handler profile and seed/log position,
- actor/session/turn id,
- relevant receipt refs and policy refs.

The goal is to identify the first semantic boundary where behavior changed, not a flood of downstream differences.

## Integration points

Executable transcripts pin initial state, handler profile, seed, policy refs, and expected trace/receipt patterns.

Evaluation cache keys include handler profile id, deterministic seed/config, initial state hash, dependency closure, and policy refs.

Remote artifact sync records dependency fetch/verification/admission effects so replay can validate remote execution setup.

Typed storage replay uses fixture snapshots or recorded storage responses; production storage reads are not deterministic unless recorded.

Distributed job DAG execution uses deterministic local/profiling/chaos profiles for tests and record/replay for production incidents.

Upgrade sessions require selected deterministic transcripts or recorded playback checks before cutover where applicable.

## Policy and evidence

Record/replay logs and snapshots may contain sensitive data and capabilities. Access requires policy admission. Replay must not treat recorded capabilities as ambient authority outside the recorded scope. Receipts should identify the profile, seed/log hash, initial state hash, dependency closure, and final state hash.

## Open Questions

- What is the exact first scheduler key and how should priorities be represented?
- Which state components must be in the first snapshot milestone versus referenced by fixture ids?
- Should replay compare full trace records byte-for-byte or compare selected deterministic fields with ignored metadata?
- How should concurrent remote deliveries be normalized into deterministic queue order during record mode?
