# Fabric transport deltas

## ADDED Requirements

### Requirement: Live Raft Iroh ingress authenticates actual peer identity and isolates remote connection failures
r[molten.fabric_transport.raft_peer_auth_and_isolation] The cross-process Iroh listener MUST derive `observed_peer_context_ref` from the actual authenticated remote `EndpointId` rather than echoing its own expected context, `received_event` MUST verify that the authenticated remote identity matches `envelope.from`, and per-connection handshake stalls or peer timeouts MUST be isolated without terminating `ReplicaIngressPump`.

#### Scenario: Spoofed envelope.from is rejected
- GIVEN an authenticated QUIC peer sends a Raft envelope whose `from` field names a different voter
- WHEN `received_event` validates the incoming message
- THEN the message MUST be rejected and counted as an authentication failure without reaching the Raft state machine.

#### Scenario: Stalled remote handshake does not kill replica ingress
- GIVEN an untrusted or partitioned peer opens a connection and stalls during handshake or close
- WHEN `ReplicaIngressPump` polls ingress
- THEN the stalled connection MUST time out and drop cleanly while the replica continues serving healthy peers.
