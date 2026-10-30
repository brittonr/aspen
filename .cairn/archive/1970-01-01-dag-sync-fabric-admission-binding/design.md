## Context

`DagTransportPort`, `DagContentVerificationPort`, `DagAuthorityPort`, resource reservations, durable progress and the existing shell remain the mechanism owners. The missing contract is cross-extension admission, not a replacement graph protocol.

## Decision

The `dag_sync` owner exports a descriptor for one `sync` operation, canonical request and receipt schema identities, external-effect/recorded-replay semantics, explicit authority/resource classes and required non-claims. Profile and conformance references are supplied by the admitting composition and validated rather than synthesized as fixture constants.

Federation resolves the descriptor with the normal fabric registry and system-tier admission. Its callback emits a generation/incarnation-fenced typed effect, and the dispatcher routes only a host-owned successful callback. The federation router calls the existing DAG shell without bypassing any of its ports.

## Boundaries

A binding does not authorize a peer, resource, object, local registry mutation, or application merge. Every invocation still obtains current DAG authority and resource observations and verifies content. Simulation uses the same descriptor resolution and shell with different mechanisms.

## Validation

Exercise accepted dispatch and deny stale generation, wrong binding, missing authority/resource admission, corrupted content and mismatched request identity. A descriptor-only or synthetic-loopback check does not establish live networking.
