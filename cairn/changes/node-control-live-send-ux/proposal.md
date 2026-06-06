# Proposal: Node Control Live Send UX

## Summary
Add an external live send command for node-control ingress so operators can send canonical requests over real `iroh-gossip` using a bound live ticket instead of relying on loopback-only tests.

## Motivation
Live serve/listener support can receive real gossip events, but the operator-facing sender workflow still lacks a command that consumes a bound live ticket, publishes canonical ingress bytes, and records send evidence. Remote/live control needs a reproducible send path that remains separate from authority, policy, resource, idempotency, and provenance gates.

## Scope
- CLI `molten node control-ingress-live-send` that consumes a request and bound live ticket.
- Canonical `node-control-live-send-receipt-v1` artifacts that bind receiver ticket, endpoint/address evidence, envelope ref, transport receipt ref, diagnostics, and non-authority checks.
- Live send implementation that joins the receiver's real `iroh-gossip` topic from ticket endpoint/address evidence and publishes canonical envelope bytes.
- Unit coverage for a bounded real live sender/listener workflow and CLI coverage for fail-closed offline tickets without endpoint addresses.

## Out of Scope
- Relay configuration management beyond ticket address evidence.
- Treating live send, tickets, endpoint ids, or neighbor events as operation authority.
- Replacing durable inbox, authority delegation, delivery idempotency, resource, policy, or provenance gates.
