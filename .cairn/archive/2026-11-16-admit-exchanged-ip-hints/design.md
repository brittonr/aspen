## Context

`live_ticket_address_refs` and `endpoint_locators` render every Iroh address with `to_string`; the importers parse `SocketAddr` back. `SocketAddrV6`'s text form keeps a numeric scope but drops the flow label. The gossip endpoint binds IPv4 loopback today, so no supported path exchanges scoped IPv6 hints.

## Decisions

### Decision: One admitted domain for exchanged hints

**Choice:** `admit_ip_hint` rejects unspecified and multicast addresses and, for IPv6, a nonzero flow label, a nonzero scope id and link-local addresses. Both exchange boundaries call it.

**Rationale:** Rejecting the flow label avoids a round-trip claim the text form cannot keep. A scope id is host-local, so honouring it on the receiver could select an unrelated interface; no consumer needs scoped or link-local peers, so the boundary rejects them rather than adding a resolution policy.

### Decision: Producers omit, consumers reject

**Choice:** Exporters skip local addresses outside the domain; importers fail with the rejected field named.

**Rationale:** A host's link-local addresses are not usable cross-host hints, so dropping them keeps tickets valid on hosts that have them. A received hint outside the domain means the peer did not follow the contract, which must not be silently reinterpreted.

## Risks / Trade-offs

- A deployment that relied on dialing link-local or scoped peers through tickets would lose that path; none is supported.
- Local bind configuration keeps its separate explicit-unicast rule and may use scoped addresses.
