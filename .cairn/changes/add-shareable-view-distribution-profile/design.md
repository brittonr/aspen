## Context

Molten owns workload-neutral content, graph synchronization, transport sessions, authority boundaries, retention, simulation, and distributed references. Tile owns Shareable View semantics and user operations.

This change adds a narrow composition profile. It does not add a second surface protocol or a UI runtime to Molten.

## Goals

- Transfer one complete Shareable View snapshot and dependency closure by canonical reference.
- Reuse authorized complete members without global cache disclosure.
- Preserve snapshot-first live state and action ordering.
- Route fork requests without defining clone semantics.
- Use the same pure transitions in deterministic simulation and live adapters.

## Ownership

| Concern | Owner |
| --- | --- |
| Shareable View manifest, state schema, action meaning, and presentation | Tile and producer application |
| Canonical interchange adapter | Optional Kamacite profile |
| Content manifest, chunk verification, DAG synchronization, and partial progress | Molten |
| Transport sessions and Iroh adapter behavior | Molten shell |
| Read, subscription, action, and fork authority facts | Caller policy and admitted authority adapters |
| Retention, purge, and local storage | Molten consumer policy |
| UI rendering and host interaction | Tile receiver |

## Decision

### Profile boundary

A `ShareableViewDistributionRequest` identifies one external manifest artifact, one exact external schema profile, one dependency-root reference, one privacy domain, one issuer domain, one requested mode, and finite transfer resources.

Molten validates the request shape and profile binding. It treats application state, action descriptors, presentation profiles, and fallback meaning as opaque typed members owned by the external contract.

The profile rejects renderer handles, file descriptors, sockets, endpoint secrets, credentials, callbacks, mutable backend objects, and undeclared executable members.

### Closure model

The external manifest supplies a complete ordered member inventory. Each member has an external role, canonical reference, declared byte length, privacy class, and required or optional status.

Molten maps the inventory into existing content-manifest and DAG-sync inputs. The mapping retains external references and Molten content references as separate nominal domains.

A complete result requires every required member to pass length, position, chunk, whole-object, request, profile, generation, privacy, and authority checks.

Missing optional presentation members can produce a typed degraded result. Missing required state or schema members deny exposure.

### Authority before availability

The shell obtains current caller, object, privacy, issuer, policy, resource, and revocation observations before it queries local availability.

A denied caller learns no member presence, hit, size, age, peer, locator, or storage-path fact. Public receipts omit reusable content references and hit details.

Read authority applies to each reveal. Subscription, semantic action, and fork requests require separate capabilities.

### Receiver-local reuse

The implementation reuses the existing content store. It does not add a general cache service.

Retained members live in a namespace selected from privacy domain, issuer policy, profile, and consumer retention class. Equal bytes in different privacy domains do not produce shared externally visible storage state.

A compatible request can omit already verified members only after current authority and exact profile checks. Corrupt, stale, revoked, expired, or policy-incompatible members return to transfer or denial.

The feature remains disabled until a measurement compares transferred bytes, completion time, storage, CPU, memory, and privacy cost for real Shareable View workloads.

### Live mode

A live session binds the external object, snapshot, producer generation, stream, current state revision, action catalog, privacy domain, authority epoch, and transport generation.

The first accepted record is a complete externally validated snapshot. Deltas require exact predecessor sequence and revision facts. A gap, generation change, privacy change, or authority change requests a replacement baseline or closes the session.

Molten transports semantic action requests and outcomes as external typed envelopes. It does not interpret application action meaning.

Ordered action admission and outcome observations remain on the reliable control lane. Replaceable visual or presentation data cannot evict them.

An uncertain mutating action remains uncertain. Reconnect does not replay it.

### Fork routing

A fork request binds the exact external object, snapshot, owner generation, requested fork profile, caller capability, and client operation identity.

Molten resolves and routes the request to the current admitted owner. It returns the owner result as a typed external envelope.

The route does not create state, choose clone policy, activate a returned object, or grant read and mutation authority for that object.

### Functional core and shell

The pure core owns request validation, role mapping, authority-order plans, closure progress, reuse admission, live transitions, gap repair, action uncertainty, fork routing plans, and receipts.

The shell owns content reads and writes, Iroh sessions, local stores, authority observations, clocks, timeouts, cancellation, transport, retention effects, and cleanup.

### Receipts

Internal receipts bind external manifest identity, profile, request, closure counts, verified bytes, reuse counts, live state, action outcomes, fork route, authority classes, privacy classes, and non-claims.

Public projections use run-local pseudonymous identifiers and aggregate counts. They expose no payload, reusable digest, endpoint, credential, path, cache-hit identity, or action input.

## Data Flow

```text
external Shareable View manifest
  -> profile and authority admission
  -> complete external member inventory
  -> existing DAG-sync plan
  -> existing content-store verification
  -> complete or degraded closure result
  -> Tile receiver adapter
```

Live flow:

```text
complete snapshot
  -> admitted live session
  -> ordered state deltas and control envelopes
  -> gap, repair, close, or current state
```

## Error Model

The profile distinguishes unsupported external contract, malformed inventory, missing required member, authority denial, privacy denial, unavailable member, corruption, truncation, reordering, resource exhaustion, cancellation, timeout, disconnect, stale generation, live gap, uncertain action, fork denial, retention denial, and cleanup uncertainty.

Availability failure does not become authority denial. Transport success does not become content verification or action success.

## Testing Strategy

Positive tests cover complete closure, authorized local reuse, partial resume, live baseline and deltas, read-only actions, mutating action outcomes, fork routing, and cleanup.

Negative tests cover pre-authority availability queries, cross-domain hits, corrupt retained members, stale progress, revoked access, gaps, action replay, owner replacement, fork overreach, and receipt leaks.

Deterministic and live Iroh profiles use the same core request, closure, progress, live, action, fork, and receipt transitions.

## Alternatives

### Add a Tile-specific cache

This duplicates Molten content and retention behavior. The profile uses existing Molten mechanisms instead.

### Transfer complete pixels

This retains compatibility but cannot reuse application state, schemas, actions, or presentation dependencies.

### Use world commits

A full world commit carries unrelated runtime roots and restore semantics. Shareable View distribution requires a smaller workload-neutral closure.

### Publish peer cache inventories

This improves discovery but creates presence and correlation risks. The design rejects this mechanism.

## Risks

- Local reuse can expose content presence through timing or resource behavior.
- External schemas can drift without exact profile binding.
- Live control and replaceable state can cross under pressure.
- Retention can preserve content after application access changes.

Authority-before-lookup, privacy partitioning, reliable control lanes, current revocation checks, and explicit purge reduce these risks.
