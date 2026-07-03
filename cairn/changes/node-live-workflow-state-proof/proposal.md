## Why

The node live workflow bundle path is a state machine over handoff, verification, gate, apply, send, reconcile, ack, and import evidence. It guards enqueue and dispatch side effects, so we need proof that out-of-order, stale, failed, or mismatched workflow evidence cannot be promoted into authority, provenance, or operation execution.

## What Changes

- Add requirements for the live workflow bundle lifecycle order.
- Require proof traces that bind bundle, gate, apply, reconcile, ack, ingress, queue, and optional dispatch refs.
- Require negative evidence for failed gate/apply/reconcile/ack, mismatched operation refs, and transport-only evidence.

## Impact

- **Files**: node daemon live workflow bundle parsing, gate/apply/reconcile/ack/import logic, and node tests.
- **Testing**: ordered happy-path workflow, out-of-order denial, stale or wrong-operation denial, failed child receipt denial, and no enqueue/dispatch before gates pass.
