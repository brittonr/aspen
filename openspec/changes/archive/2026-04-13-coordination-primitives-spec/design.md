## Context

Aspen already has three coordination layers in code:

1. `crates/aspen-coordination/` exposes the primitive implementations and local manager types.
2. `crates/aspen-client-api/` defines a shared request/response surface for coordination RPCs.
3. `crates/aspen-core-essentials-handler/` handles some primitives natively while the remaining ones can fall through to plugin-backed handlers.

The current `coordination` spec does not match that shape. It captures distributed locks and a handful of model-based test requirements, but it does not tell a reader what the rest of the subsystem promises.

## Goals

- Give Aspen one umbrella coordination spec that matches the primitive families already shipped in code.
- Describe behavior in user-facing terms: exclusivity, monotonicity, FIFO ordering, TTL expiry, health filtering, bounded retries, and similar externally visible rules.
- Capture the shared RPC contract without freezing backend implementation details.
- Avoid duplicating the already-landed `distributed-lockset` domain.

## Non-Goals

- No code changes.
- No serialization or storage-schema commitments beyond externally visible behavior.
- No promise that every primitive is implemented by the same backend forever.
- No duplication of the detailed `distributed-lockset` rules.

## Decisions

### Use `coordination` as the umbrella domain

The existing repo already has `openspec/specs/coordination/spec.md`. Expanding that domain is better than adding another nearly synonymous spec such as `coordination-primitives` or `primitives`.

Reason:

- review stays anchored on the existing domain name
- future coordination changes do not need to guess which umbrella spec to modify
- lock-set detail can stay in its dedicated domain without fragmenting the rest of the subsystem

### Specify by primitive family, not by source file

The change groups requirements by user-facing primitive families:

- locks and leader election
- counters and sequences
- rate limiting
- barriers, semaphores, and read-write locks
- queues
- service registry
- worker coordination
- coordination RPC contract

Reason:

- source-file layout can move without changing the spec
- readers care about observable semantics, not module boundaries
- the same primitive may have local, RPC, and plugin-backed implementations

### Separate contract semantics from backend dispatch

The spec should say that coordination RPC requests keep one request/response contract even if some operations are served natively and others are served by plugins.

Reason:

- current code already mixes both paths
- clients should not care which backend handled a request
- backend routing is an implementation choice, not the contract itself

### Keep lock-set detail in `distributed-lockset`

The umbrella coordination spec should acknowledge lock sets as part of the subsystem, but the detailed atomic multi-resource rules remain in `openspec/specs/distributed-lockset/spec.md`.

Reason:

- that domain already exists and is more precise than a summary restatement would be
- duplicating token, canonicalization, and release rules in two specs raises drift risk

## Trade-offs

### Broader coordination spec means more overlap risk

A larger umbrella spec can drift into neighboring domains such as `distributed-lockset` or worker-health specs.

Mitigation:

- keep umbrella requirements focused on the primitive contract surface
- leave deep, specialized invariants in their dedicated domains when one already exists

### Worker coordination is library-level today

`DistributedWorkerCoordinator` is part of the coordination crate, but it is not expressed through the same client RPC surface as locks or queues.

Mitigation:

- specify worker coordination as a subsystem primitive with library-level behavior
- keep the RPC contract requirement scoped to request families that already exist in `aspen-client-api`
