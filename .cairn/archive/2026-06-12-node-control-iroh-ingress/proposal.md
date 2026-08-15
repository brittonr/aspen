# Proposal: Node Control Iroh Ingress

## Summary
Add a deterministic local-Iroh ingress surface for node control requests that feeds the existing durable file-backed control inbox.

## Motivation
Node control now has a bounded local loop and provenance gates before side effects. The next remote-facing step needs an ingress envelope that preserves the same fail-closed semantics before any request is enqueued for dispatch.

## Scope
- Canonical node-control ingress envelope and ingress receipt artifacts.
- Local-Iroh-style publish/deliver workflow into the durable control inbox.
- Peer bootstrap, authority, policy, resource, and delivery-idempotency checks before enqueue.
- CLI support for building, publishing, and delivering ingress envelopes.
- Tests covering pass, duplicate suppression, missing authority denial, and missing provenance still denying at dispatch.

## Out of Scope
- Long-lived live network listener.
- Real remote peer transport setup beyond deterministic local-Iroh-style storage.
- New side-effecting node operations.
