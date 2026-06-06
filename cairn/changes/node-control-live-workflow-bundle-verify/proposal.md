# Change: Node Control Live Workflow Bundle Verify UX

## Motivation

Live workflow bundles can be exported and imported, but operators need a safe offline check before materializing bundle members into a sender state root. Import should not be the first time malformed members, wrong peer/topic bindings, or unsupported supporting receipts are discovered.

## Proposed Change

Add `molten node live-workflow-bundle-verify` and canonical `node-control-live-workflow-bundle-verify-receipt-v1` receipts. Verification parses the bundle, recomputes member refs, checks ticket/admission/grant bindings, validates optional expected node/topic/endpoint/peer/operation/scope/freshness/revocation bounds, and classifies unsupported supporting receipt kinds.

## Non-Goals

- Verify receipts are not authority or provenance.
- Verification does not import member artifacts or satisfy sender/receiver live-send gates.
- Verification does not replace bundle import validation; import remains fail-closed and repeats the same binding checks.
