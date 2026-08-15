# Change: Node Control Live Send Diagnostics

## Motivation

Live send now supports retry, duplicate suppression, and importable ticket/grant artifacts, but denial receipts need clearer operator-facing causes. In particular, wrong tickets, stale peer admissions, missing sender-side imports, unsupported addresses, operation-id mismatches, and transport join/publish failures should be distinguishable from one another without re-running the command.

## Proposed Change

Polish node-control live-send diagnostics:

- Add expected receiver node/topic/endpoint guards to `control-ingress-live-send`.
- When a sender state root is supplied, preflight imported peer-admission and authority-grant refs before opening live transport.
- Emit deterministic diagnostics suggesting `live-ticket-import` or `authority-grant-import` when moved evidence is missing or malformed in the sender state root.
- Extend live-send receipt checks so operation-id mismatch, address availability/support, sender state-root evidence, and join/publish success are explicit check labels.
- Preserve fail-closed behavior: diagnostics deny before transport when local evidence or ticket bindings are wrong.

## Non-Goals

- Diagnostics do not weaken receiver-side admission; receivers still validate peer bootstrap, authority, policy/resource, delivery idempotency, and provenance before enqueue.
- Sender-side import preflight does not make transport or import receipts authority.
- This change does not add a new transport protocol or live replay guarantee.
