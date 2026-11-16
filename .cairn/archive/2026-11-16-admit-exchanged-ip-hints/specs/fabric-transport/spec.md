# Fabric Transport Delta

## ADDED Requirements

### Requirement: Exchanged IP hints have an explicit admitted domain
r[molten.fabric_transport.ip_hint_domain] IP connection hints exchanged between hosts MUST be limited to unicast IPv4 and unicast IPv6 with zero flow label and zero scope id, excluding unspecified, multicast and link-local addresses. Producers MUST NOT publish a hint outside that domain, consumers MUST reject one, and every admitted hint MUST round-trip its address, port, flow label and scope. Local bind configuration MAY follow a separate policy, and admitting a hint MUST NOT grant identity, membership or authority.

#### Scenario: A sender's scope does not select a receiver interface
- GIVEN a link-local IPv6 hint carrying the sender's numeric interface index
- WHEN the sender exports hints or the receiver imports that hint
- THEN the sender omits it and the receiver rejects it, naming the scope
- AND the receiver never dials through its own interface with that index.

#### Scenario: Admitted hints round-trip
- GIVEN a unicast IPv4 hint or an unscoped, flow-label-free unicast IPv6 hint
- WHEN it is exported as text and imported
- THEN its address, port, flow label and scope are unchanged.
