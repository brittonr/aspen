# Change: Node Control Live Workflow Bundle Import/Export

## Motivation

Operators can now export/import live tickets and authority grants separately, and live-send diagnostics explain missing sender-side evidence. Multi-file handoff is still awkward: a sender often needs the receiver live ticket, matching peer admission, matching authority grant, and relevant operational receipts as one auditable artifact.

## Proposed Change

Add a live workflow bundle handoff artifact and commands:

- `molten node live-workflow-bundle-export`
- `molten node live-workflow-bundle-import`

The bundle carries the live ticket, peer admission, authority grant, and optional supporting receipts. Export receipts bind the bundle members and classify malformed bundles. Import receipts validate the same node/topic/endpoint/peer/operation/scope/freshness/revocation checks as the individual import commands, then import the underlying member artifacts into the sender state root.

## Non-Goals

- Bundles are not authority or provenance.
- Bundle import receipts do not satisfy receiver-side admission, authority, policy/resource, delivery-idempotency, or provenance gates.
- Bundles do not introduce a new transport or replace live-send retry/duplicate receipts.
