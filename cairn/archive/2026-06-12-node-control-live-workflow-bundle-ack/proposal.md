# Change: Node Control Live Workflow Bundle Ack UX

## Motivation

Reconcile receipts prove receiver-side evidence, but the sender still needs a portable acknowledgement artifact to import that receiver evidence into its own durable ledger. Without an ack bundle, operators must manually copy ingress, queue, control, and reconcile receipts and risk losing the binding between the original apply receipt and the receiver outcome.

## Proposed Change

Add `molten node live-workflow-bundle-ack-export` and `molten node live-workflow-bundle-ack-import`. Ack export packages an apply receipt, optional send receipt, receiver ingress receipt, queue receipt, optional control receipt, and reconcile receipt into canonical `node-control-live-workflow-bundle-ack-v1`. Ack import validates the package against optional expected bundle/envelope/operation/request refs and imports the ack plus member receipts into the sender state root.

## Non-Goals

- Ack bundles and ack import/export receipts do not satisfy authority, peer bootstrap, policy/resource, provenance, delivery-idempotency, sender-import, or receiver-ingress gates.
- Ack import does not perform live network sends, receiver dispatch, or control-loop execution.
- Ack decision reflects package validity, not whether the receiver control outcome passed; receiver denials remain recorded as receiver diagnostics.
