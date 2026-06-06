# Design: Node Control Live Workflow Bundle Import/Export

## Bundle artifact

`node-control-live-workflow-bundle-v1` is a canonical Preserves artifact containing:

- the receiver `node-control-live-ticket-v1`;
- the matching `node-control-live-peer-admission-v1`;
- the matching `node-control-authority-grant-v1`;
- optional supporting receipts such as live-send, listener, workflow, and import receipts;
- canonical member refs and bundle checks.

The bundle record includes the member values and refs so importers can recompute the bundle hash and reject ref/member mismatches before importing.

## Export receipt

`node-control-live-workflow-bundle-export-receipt-v1` binds the bundle ref, ticket ref, peer admission ref, grant ref, included receipt refs, diagnostics, and checks. Export validates ticket/admission/grant binding and only marks the receipt pass when the included receipts are known live workflow receipt kinds.

## Import receipt

`node-control-live-workflow-bundle-import-receipt-v1` validates bundle member bindings and reuses the same expected node/topic/endpoint/peer/operation/scope/freshness/revocation checks as `live-ticket-import` and `authority-grant-import`. Missing fields, malformed member records, and member/ref mismatches fail closed before importing member evidence. On pass, import materializes the ticket, peer admission, grant, bundle, and included receipts into the target state root. On deny, the import receipt is still imported but bundle members are not treated as admitted authority/provenance.

## Authority boundary

The bundle and bundle import/export receipts are operational evidence only. Receiver-side live ingress still requires original peer admission, authority grant, policy/resource evidence, idempotency evidence, and payload provenance before enqueue or side effects.
