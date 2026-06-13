# Proposal: Node Control Live Peer Tickets

## Summary
Add canonical live endpoint ticket and peer admission evidence so live node-control peer bootstrap is resolved from durable receipts instead of synthetic local refs.

## Motivation
Live Iroh ingress already separates transport from authority, but peer bootstrap evidence is still represented by generic refs. Operators need a concrete ticket/admit workflow that binds a remote peer to a node-control live topic before enqueue, while keeping authority grants and payload provenance separate.

## Scope
- Canonical `node-control-live-ticket-v1` artifacts for node/topic/live endpoint bootstrap.
- Canonical `node-control-live-peer-admission-v1` receipts for admitting a peer against a live ticket.
- CLI helpers for `live-ticket-export`, `live-peer-admit`, and `serve --live-ticket-out`.
- Live ingress pre-enqueue validation that resolves peer bootstrap refs to admitted peer ticket evidence.
- Fail-closed coverage for unknown, wrong-peer, wrong-node/topic, not-yet-valid, and expired admissions.

## Out of Scope
- Public relay configuration UX beyond recorded endpoint/address evidence.
- Treating tickets or peer admissions as operation authority.
- Replacing authority delegation, policy/resource, delivery idempotency, or provenance gates.
