## Why

Molten nodes need a safe way to discover peers, establish identity, negotiate compatible features, exchange capability offers, and join swarms or protocol/control-plane groups. Without an explicit bootstrap and negotiation model, remote sync, gossip, docs, blobs, choreography, and job execution can each invent incompatible handshakes.

## What Changes

- Define node bootstrap records, peer discovery inputs, handshake envelopes, and join-admission receipts.
- Negotiate protocol versions, artifact registry support, schema identity support, effect handler profiles, transport capabilities, resource limits, replay support, and policy requirements.
- Exchange capability offers without granting ambient authority.
- Gate swarm/topic/doc/protocol/job group joins through policy and authority checks.
- Record peer identity, feature negotiation, selected transports, capabilities, and denial reasons in receipts.
- Integrate bootstrap with Iroh endpoint setup, remote artifact sync, catalog visibility, Raft control-plane groups, and deterministic record/replay.

## Impact

This makes multi-peer operation explicit and auditable. The first milestone can implement a local/loopback handshake manifest and validation logic before real network join behavior.
