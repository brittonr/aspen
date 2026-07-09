## Context

Molten already has effect manifests inspired by Unison abilities. This change turns the adaptation into a stronger boundary: all executable artifacts and handlers must agree on declared operations, schemas, capability needs, resource needs, replay semantics, and profile identity.

## Design

### Effect flow

```text
artifact ref
  -> effect-manifest-v1
  -> requested handler profile
  -> capability/policy/resource/provenance/source-gate checks
  -> handler-profile-admission-receipt-v1
  -> execution may issue declared effect requests only
```

The effect manifest is a contract about possible requests. It is not a grant. The handler admission receipt proves that a specific runtime profile may handle that manifest under current policy and capability context.

### Handler profiles

Profiles should be explicit records for contexts such as:

- production handlers with real side effects;
- local deterministic handlers for tests and transcripts;
- chaos/fault handlers for simulation;
- profiling handlers that measure without changing semantics;
- replay handlers backed by recorded effect logs.

Each profile binds supported effect ids, operation schemas, resource bounds, determinism/replay class, policy refs, and evidence refs.

### Replay binding

Replay, evaluation-cache, transcript, and remote execution receipts must include the exact effect manifest and admitted handler profile refs. A cache hit or replay pass under one profile cannot satisfy another profile unless an explicit compatibility receipt says so.

### Functional core and shell

Pure cores compare manifests to profiles, validate operation schemas, detect undeclared requests, and compute replay/cache keys. Shells call real handlers, persist receipts, enforce capability/resource/policy gates, and render diagnostics.

### Non-goals

- Do not adopt Unison syntax, typechecker behavior, runtime effect semantics, or generalized algebraic effects.
- Do not grant capabilities from effect declarations.
- Do not let handler names or process environment choose behavior implicitly.
- Do not let cache/replay skip current capability, revocation, resource, or policy checks.