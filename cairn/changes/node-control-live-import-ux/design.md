# Design: Node Control Live Import UX

## Receipt model

This change adds two receipt classes:

- `node-control-live-ticket-import-receipt-v1` binds the local state-root profile, ticket ref, node, topic, endpoint, optional peer-admission ref, optional peer id, as-of sequence, imported refs, diagnostics, and checks.
- `node-control-authority-grant-import-receipt-v1` binds the local state-root profile, grant ref, peer, node, operations, target/resource scopes, as-of epoch, imported refs, diagnostics, and checks.

Receipts are imported into the local node ledger whether they pass or deny. Ticket, admission, and grant artifacts are imported only when validation passes.

## Ticket and admission import

`live-ticket-import` parses the live ticket and validates the schema/version through the existing parser. Optional expectations bind the ticket to an expected node id, topic, live endpoint id, and peer id. When a peer-admission artifact is supplied, the importer validates that it is a passing `node-control-live-peer-admission-v1`, that it references the imported ticket, that node/topic match the ticket, and that its sequence/expiry bounds cover the supplied `--as-of-sequence`.

## Authority grant import

`authority-grant-import` parses the grant through the existing schema/version parser. Optional expectations bind peer id, node id, required operations, target scope, and resource scope. The importer also rejects grants that are not yet valid for `--as-of-epoch`, are expired for that epoch, or carry revocation refs.

## Non-authority boundary

The import receipts are operational evidence only. A passing ticket import does not become peer bootstrap authority; receiver ingress still requires a live peer-admission ref. A passing authority-grant import receipt does not replace the grant itself, and neither import receipt supplies policy/resource/provenance evidence.

## CLI workflow

Operators can export/admit on the receiver state root, move the ticket/admission/grant files, and import them into a sender state root before running `control-ingress-live-send`. The import receipts provide deterministic diagnostics for stale tickets, wrong peer/node/topic, wrong operation/scope, and expired/revoked grants.
