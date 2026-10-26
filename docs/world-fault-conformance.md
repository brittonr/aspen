# World crash and concurrency conformance

Molten has a bounded conformance rail for world mutation failures. The rail tests semantic phases, restart read-back, and explicit concurrent schedules.

The rail does not replace any subsystem decision core. Each owner classifies its own durable observations and recovery result.

## Mutation inventory

The inventory is closed and versioned. An unknown world mutation makes inventory validation fail.

| Mutation | Owner | Linearization point | Durable record | Recovery entry |
|---|---|---|---|---|
| Capture | World Commit | Immutable object publication | Capture receipt | Capture read-back |
| Head | World Head | Head transaction commit | Head transition receipt | Head read-back |
| Promotion | World Promotion | Promotion transaction commit | Promotion and reservation set | Promotion read-back |
| Witness | World Head Witness | Witness record commit | Independent witness record | Witness read-back |
| Outbox | World Promotion Outbox | Attempt record commit | Effect attempt record | Attempt recovery |
| Replication | World Distribution | Replica availability commit | Reachability receipt | Replication read-back |
| Import | World Replay | Capsule availability commit | Replay import receipt | Import read-back |
| Retention | World Retention | Retention root commit | Retention root inventory | Retention read-back |
| Garbage collection | World Garbage Collection | Garbage-collection plan commit | Garbage-collection plan | Replan from current facts |

The witness row is unsupported in the local profile. It stays in profiles and receipts until an independent witness owner exists.

## Semantic fault phases

Each supported row has these phases:

1. `uninterrupted`
2. `before-submit`
3. `after-possible-submit`
4. `after-durable-write`
5. `before-response`
6. `lost-response`
7. `process-restart`
8. `recovery-read-back`

Adapter hooks map local implementation points to these names. Source lines and wall-clock timing do not define a semantic phase.

A lost response after possible submission enters the uncertain path. The harness never changes that result into a safe retry.

### Committed-change subscriber

<!-- r[impl molten.observations.stream] -->

The v2 local profile requires `committed-subscriber:publication-before-ack` at
`before-response`. It checkpoints one prior committed event, publishes three
events in one later revision, receives a one-event page without acknowledging it,
then discards the volatile subscription cursor and resumes from the acknowledged
checkpoint. Every published event must be redelivered in order, including the
unacknowledged prefix; stable-identity deduplication must yield exactly the
published sequence. A cursor ordinal is the inclusive lower bound of the next
event within its revision, so bounded pages cannot skip an ordinal.

This is a deterministic in-process subscriber restart, not an operating-system
process crash or a persistence-adapter test. The stream remains owned by the
producer throughout. The 17-restart budget covers 16 existing world-adapter
restarts and this one modeled subscriber restart. The case admits 2–64 events
and a positive page bound smaller than the publication; there is no wall-clock
or delivery-election change.

The subscriber result fails if its checkpoint advanced before acknowledgement,
delivery is missing or reordered, the service/epoch/phase or event identities
cross, the modeled restart is absent, or deduplication differs. Missing subscriber
evidence cannot produce an aggregate pass. Receipt validation independently
rechecks that evidence and rejects a forged passing disposition.

## Recovery classes

Owner cores return one of these classes:

- `already-complete`
- `safe-to-retry`
- `superseded`
- `conflict`
- `uncertain`
- `denied`
- `corrupt`
- `manual-review`

The harness compares the owner result with the reviewed case. It also applies conservative observation rules.

An `already-complete` result requires exact durable state and record read-back. Missing state cannot become success.

Corrupt state requires `corrupt` or `manual-review`. Contradictory state requires `conflict` or `manual-review`.

Cleanup needs complete applied state and an independent witness. A focused conformance receipt never grants cleanup authority.

## Concurrent schedules

Schedules bind operation identities, expected generations, pre-state identities, nodes, and interleaving points. Every schedule has two competing operations.

The standard profile covers head, promotion, witness, outbox, import, replication, retention, and garbage collection.

Each schedule uses these points:

1. `prepare`
2. `current-fact-recheck`
3. `before-linearization`
4. `durable-read-back`

The profile reuses Fabric Simulation scheduler choices. Wall-clock timing cannot select the winner.

At most one operation can linearize for one generation and pre-state. Promotion and outbox schedules also allow at most one effect release.

## Restart boundary

The imperative shell controls interruption and process restart. It reopens local node state before it supplies durable read-back.

The shell then calls the owning decision port. It does not create success, compensation, conflict, or retry decisions.

Transactional Reconciliation Core classifies promotion persistence. Unknown commit outcomes stay quarantined until read-back supplies exact facts.

## Rollback boundary

A restored local image can roll back the head and its generation together. The local profile cannot detect that condition by itself.

Strong rollback detection needs independent state that does not roll back with the local store. The current witness row remains unsupported.

## Typed profile

The reviewed Nickel profile is at:

`config/world-faults/profiles/local-deterministic.ncl`

The generated JSON is at:

`config/world-faults/generated/local-deterministic.json`

The generated export is checked against Nickel, and the executed smoke scenario
checks its v2 Rust profile identity. Positive and negative fixtures cover the
subscriber role, phase and bounds alongside missing world phases, wall-clock
winners, zero limits, and witness overclaims.

## Receipt meaning

The canonical Preserves receipt binds:

- the source revision;
- the inventory and profile identities;
- adapter and schedule identities;
- named limits;
- cases and durable observations;
- owner decisions;
- schedule outcomes;
- unsupported rows;
- subscriber phase, acknowledged checkpoints, published and delivered events,
  deduplicated sequence, modeled restart count and diagnostics;
- all required non-claims.

A passing receipt proves bounded agreement for the exercised cohort. It does not prove universal crash safety or physical power-loss behavior.

Profiles, profile identity domain, canonical receipt record and receipt identity
domain use v2 for the required subscriber role. Old v1 receipts are historical
evidence, not acceptance of the v2 profile. Stream event/cursor identity formats
are unchanged; the cursor comparison repair restores their next-event contract.

It does not prove storage correctness. It does not establish release eligibility.
