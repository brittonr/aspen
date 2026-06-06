# Design: Node Control Live Send UX

## Artifacts
- `node-control-live-send-receipt-v1` records the send decision, live transport profile, topic, from peer, destination node, receiver ticket ref, receiver endpoint id, receiver address evidence, envelope ref, optional transport receipt ref, diagnostics, and checks.
- Existing `node-control-live-transport-receipt-v1` remains the transport attempt evidence for gossip publish/receive. The send receipt binds the publish transport receipt to the receiver ticket and endpoint/address evidence.
- `node-control-live-workflow-receipt-v1` binds the receiver ticket, peer admission, authority grant, send receipt, receive transport receipts, optional listener receipt, service-run receipt, diagnostics, and checks for ticket/admission/authority/send/listener consistency.

## Workflow
1. A receiving node runs `molten node serve --live-iroh --live-ticket-out <ticket>` to export a bound live ticket with endpoint address evidence.
2. A peer builds a normal node-control request and obtains separate peer-admission and authority evidence.
3. `molten node control-ingress-live-send <request> <ticket>` builds a canonical live ingress envelope for the ticket node/topic, joins the real Iroh gossip topic using the ticket endpoint/address evidence, publishes canonical bytes, and emits a send receipt.
4. The receiving listener accepts the gossip event through the existing live receive boundary, then durable ingress gates peer bootstrap, authority delegation, policy/resource, idempotency, and queue side effects.
5. `molten node live-workflow-bundle` ties the ticket/admission/grant/send/receive/listener/service evidence into a runbook receipt that can pass or deny with diagnostics.

## Gate ordering
Live send and workflow evidence prove only that canonical bytes were published and that the operator runbook evidence is internally consistent. They are not peer bootstrap, operation authority, resource policy, idempotency, or payload provenance. Offline tickets without endpoint addresses fail closed with a send receipt and no transport receipt.
