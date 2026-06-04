# Design: Node Control Live Peer Tickets

## Artifacts
- `node-control-live-ticket-v1` binds node id, node identity ref, logical endpoint id, live Iroh endpoint id, topic, exported addresses, policy refs, evidence refs, and checks stating the ticket is bootstrap evidence only.
- `node-control-live-peer-admission-v1` binds decision, peer id, ticket ref, node id, topic, sequence/expiry, policy refs, evidence refs, diagnostics, and checks stating authority is still required.

## Workflow
1. `molten node live-ticket-export` writes/imports a deterministic ticket for the node live topic.
2. `molten node live-peer-admit` imports a ticket, verifies it matches the local node identity/live endpoint derivation, and writes/imports a peer admission receipt.
3. `molten node serve --live-iroh --live-ticket-out` writes the bound listener ticket with observed endpoint addresses.
4. Live ingress delivery resolves peer bootstrap refs to admitted peer admission receipts before delivery idempotency or queue side effects.

## Gate ordering
Live peer ticket admission is a bootstrap gate only. It runs before authority delegation, policy/resource checks, delivery idempotency, and durable queue side effects. A ticket/admission never grants operation authority; live authority refs still have to resolve to `node-control-authority-grant-v1`, and install/run payload trust remains under provenance gates.
