# retention-remote-clearance-live-multihost-ux

## Summary

Add operator-facing multi-host live remote-clearance commands that split the existing loopback flow into request send, peer response send, and requester import/workflow steps.

## Motivation

The live loopback workflow proves the node-control evidence boundary locally, but operators need the same request/respond/import sequence across two live node roots without changing retention safety semantics. Live transport receipts should help diagnose delivery and binding problems, while deletion safety still depends on importing a passing `retention-remote-gc-clearance-import-v1` response.

## Scope

- Add CLI support for sending clearance request refs over node-control live transport to a peer.
- Add CLI support for producing a peer response and sending its ref back over node-control live transport.
- Add CLI support for assembling the final live workflow evidence from request, response, import, send, receive, and ingress receipts.
- Preserve evidence-only boundaries: live transport receipts do not grant authority, policy, resource, provenance, execution, source-gate, remote-GC trust, or destructive clearance.

## Non-goals

- No new transport authority model.
- No replacement for `retention-remote-gc-clearance-import-v1` as the destructive-admission gate.
- No requirement that live transport carries artifact bytes; request and response artifacts remain canonical values that may be moved by files, bundles, or later protocols.
