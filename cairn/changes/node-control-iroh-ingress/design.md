# Design: Node Control Iroh Ingress

## Overview
The ingress surface is an explicit two-phase local-Iroh model:

1. Build/publish a canonical `node-control-ingress-envelope-v1` that embeds a canonical `node-control-request-v1` and remote admission metadata.
2. Deliver the stored envelope to a state root, validate remote ingress evidence, run delivery idempotency, and enqueue the embedded request through the existing durable inbox path.

This keeps the control loop, operation dispatch, and provenance gates unchanged. Remote ingress only decides whether a request may enter the local inbox.

## Records
- `node-control-ingress-envelope-v1` binds transport profile, topic, from peer, target node, sequence, operation ref, request ref, embedded request, peer bootstrap refs, authority refs, policy refs, resource refs, and evidence refs.
- `node-control-ingress-receipt-v1` binds phase (`publish`, `deliver`, or `duplicate`), decision, envelope ref, request ref, operation ref, optional idempotency receipt ref, optional queue receipt ref, diagnostics, and checks.

## Admission and idempotency
Delivery fails closed before enqueue unless the envelope has explicit peer bootstrap, authority, policy, and resource refs. For admitted envelopes, `delivery_idempotency` is evaluated with the remote-topic scope for the target node and topic. First deliveries enqueue; duplicate deliveries suppress side effects; stale, gap, or conflict decisions deny before enqueue.

## Provenance interaction
Ingress does not bypass operation gates. Install and run requests still require admitted provenance evidence during dispatch or loop processing before side effects.

## CLI
`molten node control-ingress-build`, `control-ingress-publish`, and `control-ingress-deliver` expose the deterministic local workflow. Operators can write receipts to files and then use `molten node run-loop` to dispatch admitted requests.
