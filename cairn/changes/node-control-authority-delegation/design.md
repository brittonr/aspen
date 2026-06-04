# Design: Node Control Authority Delegation

## Artifacts
- `node-control-authority-grant-v1` binds peer id, node id, allowed operations, target/resource scope, epoch, optional expiry, policy refs, revocation refs, evidence refs, and checks.
- `node-control-authority-receipt-v1` records a live ingress authority decision for an envelope/request pair and the admitted grant ref, if any.

## Live ingress gate
Live ingress delivery now performs authority delegation validation before idempotency or queue side effects:
1. Read each live envelope authority ref from the node ledger.
2. Parse it as a node-control authority grant.
3. Require peer, node, operation, epoch/expiry, target scope, resource scope, and revocation checks to pass.
4. Admit if at least one grant passes; otherwise emit a deny authority receipt and deny the ingress receipt.

Local file ingress keeps its existing synthetic authority refs for deterministic test/control workflows. Live transport remains non-authority evidence only; peer bootstrap, policy, resource, delivery idempotency, provenance, and side-effect dispatch gates remain separate.

## CLI fixture
`molten test node authority-grant-fixture` creates canonical grant fixtures and can import them into a node state root so live ingress tests/operators can reference admitted grant hashes.
