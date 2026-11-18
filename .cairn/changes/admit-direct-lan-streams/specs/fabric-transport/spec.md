# Fabric transport deltas

## ADDED Requirements

### Requirement: Direct-LAN streaming is explicitly admitted
r[molten.fabric_transport.direct_lan] The direct profile MUST bind actual authenticated EndpointId to peer context, exact service/protocol/ALPN, remote role and generation before payload or protected local resource opening. Current operation authority MUST be separately resolved. Relay/discovery MUST remain unsupported and early-data lifecycle mutation MUST deny.

#### Scenario: Wrong peer denies
- GIVEN an authenticated but unexpected peer
- WHEN admission runs
- THEN no protected socket opens or payload callback runs.

### Requirement: Receive admission precedes authority resolution
r[molten.fabric_transport.receive_gate] Direct inbound sessions MUST pass a deterministic bounded per-source time budget before expensive service authority resolution. Accepted handler input MUST have no public constructor. In-budget existing transition bytes MUST remain unchanged.

#### Scenario: Source budget is exhausted
- GIVEN the source consumed its injected-time budget
- WHEN another session arrives
- THEN a typed rate denial occurs before authority resolution.

### Requirement: Streaming preserves bounds and uncertainty
r[molten.fabric_transport.direct_stream_bounds] Direct streams MUST bound connection/stream counts, frame/total bytes, queued and in-flight memory, frame work, idle and total deadlines. Cancellation, half-close, reset, drain and cleanup MUST be explicit. Consumed bytes MUST NOT be silently replayed; failure without definitive ACK MUST remain uncertain. Same-host evidence MUST NOT become two-host or Kiln qualification.

#### Scenario: Acknowledgement is lost
- GIVEN bytes were consumed or submitted
- WHEN no definitive acknowledgement arrives
- THEN reconciliation evidence reports uncertainty without automatic retry.

#### Scenario: Protected opening is interrupted
- GIVEN the protected-service open future may already have performed an effect
- WHEN cancellation or a deadline drops it before it returns a socket
- THEN delivery remains uncertain despite zero transport byte counters, and cleanup observation is reported separately.

#### Scenario: The caller drops a running operation
- GIVEN a listener reservation or registered request has been consumed
- WHEN the caller drops the public operation future before terminal cleanup
- THEN no success receipt is manufactured, unobserved listener capacity is not reused, and the registered request cannot be replayed.
