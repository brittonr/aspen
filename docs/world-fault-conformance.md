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

Rust tests compare the generated profile with the Rust projection. Negative fixtures reject missing phases, wall-clock winners, zero limits, and witness overclaims.

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
- all required non-claims.

A passing receipt proves bounded agreement for the exercised cohort. It does not prove universal crash safety or physical power-loss behavior.

It does not prove storage correctness. It does not establish release eligibility.
