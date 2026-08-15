## Why

The VM topology currently exercises substantial receipt workflows, but some cross-node paths still rely on test-driver artifact transfer for evidence movement. Reviewers need one explicit VM scenario where a request is delivered through the live admitted transport between nodes, with artifact copying reserved for post-run evidence export only.

## What Changes

- Add a NixOS VM scenario for true cross-node live transport from sender to receiver.
- Bind listener, ticket, peer admission, send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate receipts into the VM run receipt.
- Add negative gates for wrong peer, wrong node, stale ticket, missing receive receipt, and log-only pass claims.
- Document that live transport VM evidence is topology-scoped and does not grant authority, policy, provenance, resource, source-gate, retention, or production-readiness trust by itself.

## Impact

The VM profile will prove that the live transport path works across node boundaries in the test topology, not just that separately generated artifacts can be copied and verified after the fact.