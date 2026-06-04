# Proposal: Node Control Authority Delegation

## Summary
Add explicit node-control authority delegation artifacts and fail-closed live ingress checks so transport identity never substitutes for operation authority.

## Motivation
Live Iroh node-control envelopes can now reach the durable inbox. They still need a concrete authority story: the sender peer must present admitted delegation evidence for the requested node, operation, scope, epoch, and revocation state before any enqueue or dispatch side effect.

## Scope
- Canonical node-control authority grant artifacts and authority check receipts.
- Live ingress pre-enqueue validation that resolves authority refs to admitted grants in the local node ledger.
- Fail-closed denial for unknown peer/grant, expired epoch, wrong operation, wrong node/target/resource scope, and revoked grants.
- CLI fixture helper for deterministic grant creation/import in tests and operator workflows.
- Receipt checks that keep authority delegation separate from transport, policy, resource, provenance, and delivery idempotency evidence.

## Out of Scope
- Real external peer bootstrap/ticket UX.
- Cryptographic delegation signatures or remote grant exchange.
- Treating local test ingress refs as durable live grants.
- Replacing install/run provenance gates.
