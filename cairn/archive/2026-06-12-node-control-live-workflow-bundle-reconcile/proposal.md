# Change: Node Control Live Workflow Bundle Reconcile UX

## Motivation

Apply/send receipts prove what the sender attempted, but operators still need a durable receiver-side acknowledgement path. Without reconciliation, a sent bundle workflow can stop at transport evidence without proving the receiver accepted the same envelope, enqueued the request, or dispatched it to a control receipt.

## Proposed Change

Add `molten node live-workflow-bundle-reconcile` and canonical `node-control-live-workflow-bundle-reconcile-receipt-v1` receipts. Reconcile reads an apply receipt, optional live-send receipt, receiver ingress receipt, optional queue receipt, and optional control receipt. It validates that refs and expected bindings line up, denies missing/wrong/stale receiver evidence, propagates receiver denial diagnostics, and prints deterministic next-step guidance.

## Non-Goals

- Reconcile receipts do not satisfy authority, peer bootstrap, policy/resource, provenance, delivery-idempotency, sender-import, or receiver-ingress gates.
- Reconcile does not perform live network sends or dispatch receiver requests.
- Reconcile does not weaken receiver-side ingress or control-loop fail-closed behavior; it only checks receipts produced by those paths.
