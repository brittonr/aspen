# Design: Node Control Live Workflow Bundle Reconcile UX

## Reconcile command

`molten node live-workflow-bundle-reconcile` reads an apply receipt and writes a `node-control-live-workflow-bundle-reconcile-receipt-v1` to stdout or `--receipt-out`. Optional inputs include `--send-receipt`, `--ingress-receipt`, `--queue-receipt`, and `--control-receipt`. Guard flags `--expected-envelope`, `--expected-operation`, and `--expected-request` fail closed when supplied refs do not match receiver evidence.

Reconcile requires the apply receipt to pass and to bind a live envelope. When the apply receipt names a send receipt, the supplied send receipt must be present, passing, and bound to the same envelope. A supplied receiver ingress receipt must be passing for a successful reconciliation, must bind the same envelope and operation, and must carry a queue receipt ref when enqueue succeeded. A supplied queue receipt must match the ingress queue/request refs. A supplied control receipt must match the receiver request ref; denial diagnostics are copied into the reconcile receipt.

## Receipt semantics

Reconcile receipts bind the apply receipt ref, bundle ref, optional send/ingress/queue/control receipt refs, envelope/operation/request refs, diagnostics, and checks. Passing reconciliation proves only that the presented receiver evidence matches the sender workflow. It is not authority, provenance, policy/resource evidence, delivery-idempotency evidence, or a replacement for the original receiver ingress/control receipts.
