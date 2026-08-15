# Proposal: Node Control Live Iroh Transport

## Summary
Add a real live Iroh gossip transport path for node-control ingress envelopes while preserving the existing durable inbox and dispatch gates.

## Motivation
The supervised runner can process deterministic local-Iroh ingress, but Molten needs the same envelope and receipt model to cross an actual Iroh gossip transport boundary. The live transport must not become an authority token or direct-dispatch path.

## Scope
- Live `iroh-gossip` node-control ingress envelopes and transport receipts.
- Async publish/receive helpers over `iroh_gossip::api` events and senders.
- Receive path stores the envelope and calls existing ingress delivery into the durable inbox.
- Local two-endpoint Iroh loopback harness and CLI command for deterministic validation.
- Tests for live transport receipts and live ingress enqueue.

## Out of Scope
- Long-running public internet listener lifecycle.
- Relay/endpoint ticket management beyond loopback bootstrap.
- New node-control operations or dispatch bypass.
