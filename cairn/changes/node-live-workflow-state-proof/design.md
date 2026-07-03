# Design: node live workflow state proof

## Scope

This change proves the node live workflow bundle state machine. It covers bundle export, offline verify, import gate, apply, optional send, receiver ingress/queue/control evidence, reconcile, ack export/import, protocol gate evidence, and final workflow receipts.

## Proof checklist

- **Proof claim**: a live workflow advances only through ordered, passing, matching evidence; failed, stale, wrong-operation, or out-of-order evidence cannot enqueue, dispatch, import, or satisfy protocol gates.
- **Out of scope**: WAN reliability, neighbor liveness, NAT behavior, and real transport delivery beyond canonical receipts.
- **Trusted assumptions**: canonical Preserves refs and existing protocol-session gate replay remain stable.
- **Positive evidence**: a complete bundle workflow binds handoff, gate, apply, send/ingress, reconcile, ack, and import refs with matching operation/request/envelope ids.
- **Negative evidence**: failed gate, failed apply, mismatched ack, wrong operation id, stale request ref, missing sender import, and transport-only receipts deny before enqueue or dispatch.
- **Canonical refs**: bundle ref, verify/gate/apply/reconcile/ack refs, ticket/admission/grant refs, envelope/operation/request refs, ingress/queue/control refs, and protocol gate refs.
- **Regeneration command**: `cargo test node`.

## Functional core

Represent workflow advancement as pure validation over parsed receipts and expected refs. The imperative shell may read ledgers and write receipts only after the pure core returns a pass decision.

## Non-goals

- No claim that Iroh transport is exactly-once or always available.
- No authority or provenance elevation from bundle, transport, reconcile, ack, or protocol gate receipts.
