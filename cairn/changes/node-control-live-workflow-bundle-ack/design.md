# Design: Node Control Live Workflow Bundle Ack UX

## Ack export

`molten node live-workflow-bundle-ack-export` reads an apply receipt, optional `--send-receipt`, receiver `--ingress-receipt`, `--queue-receipt`, optional `--control-receipt`, and a required `--reconcile-receipt`. It recomputes reconciliation from the supplied member receipts, requires the supplied reconcile receipt to match, requires receiver ingress evidence, and requires durable queue evidence when ingress passed or named a queue receipt. It writes a `node-control-live-workflow-bundle-ack-v1` artifact plus a `node-control-live-workflow-bundle-ack-export-receipt-v1`.

The ack artifact embeds the member receipt values, member refs, bundle/envelope/operation/request refs, the receiver reconcile decision, receiver diagnostics, package diagnostics, and checks. Receiver control denials remain valid ack contents: the ack package can pass while the recorded receiver decision is deny.

## Ack import

`molten node live-workflow-bundle-ack-import` reads an ack artifact and explicit `--state-root`. Optional guards `--expected-bundle`, `--expected-envelope`, `--expected-operation`, and `--expected-request` fail closed on mismatches. Import parses all member receipts, recomputes reconciliation, rejects stale or mismatched reconcile receipts, rejects incomplete receiver evidence, and then imports the ack plus member receipts into the sender ledger before emitting `node-control-live-workflow-bundle-ack-import-receipt-v1`.

## Receipt semantics

Ack artifacts and ack import/export receipts are operational evidence only. They can carry receiver outcomes, including denials, but cannot replace grants, peer admissions, provenance records, policy/resource evidence, send receipts, ingress receipts, or control receipts in their original gates.
