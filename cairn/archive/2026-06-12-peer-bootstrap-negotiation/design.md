## Context

Iroh provides endpoint, gossip, blob, and docs substrates, but Molten semantics live above transport. A peer must not become trusted simply because it connects. Nodes need an admitted handshake that binds identity, capabilities, feature compatibility, policy requirements, resource limits, and transport choices.

## Goals

- Define canonical bootstrap and handshake records.
- Authenticate peer/node identity where policy requires it.
- Negotiate feature and protocol compatibility deterministically.
- Exchange capability offers and requests without ambient authority.
- Gate topic/doc/swarm/protocol/job/control-plane joins through policy.
- Emit receipts for successful and failed negotiation.

## Non-Goals

- Do not make Iroh transport identity sufficient for Molten authority.
- Do not grant all local capabilities to connected peers.
- Do not require every deployment to use global discovery.
- Do not make feature negotiation silently downgrade security.

## Bootstrap inputs

Bootstrap may use:

- static peer configs,
- invitation tickets,
- Iroh endpoint ids,
- local discovery,
- catalog records,
- gatekeeper-issued join credentials,
- Raft/control-plane membership records.

Each input has provenance and policy refs.

## Handshake record

A handshake should include:

- local and remote node ids/key refs,
- protocol and runtime version ranges,
- supported artifact kinds and registry protocol versions,
- schema identity and Preserves boundary versions,
- supported handler profiles/effects,
- transport capabilities for gossip/blobs/docs,
- resource limits and quotas,
- replay/trace support,
- requested topics/docs/protocol groups/job pools,
- presented capabilities and attenuations,
- policy requirements and receipt refs.

The negotiated result is a canonical agreement artifact/record.

## Capability exchange

A capability offer advertises possible authority under conditions; it is not authority until accepted and admitted. Capability requests must be attenuated, scoped, expiring where possible, and recorded in receipts.

## Join admission

Joining a gossip topic, docs namespace, protocol session, job pool, or Raft control-plane group is a trust-boundary action. Policy checks identity, capabilities, version compatibility, resource limits, and provenance. Denials should be explicit and safe to replay.

## Version negotiation

Negotiation selects the highest mutually admitted feature set under policy. Downgrades that remove security, evidence, or schema requirements must be denied unless explicitly allowed by policy.

## Open Questions

- Which features are mandatory in the first compatibility vector?
- Should join agreements be short-lived leases by default?
- How should offline invitation tickets be revoked before use?
