## Why

The file-backed `molten node` lifecycle now starts, reports status, and stops with receipts, but operators still lack a durable control ingress that persists canonical requests, serializes dispatch, rejects stale process state, and imports control/suboperation evidence into the node ledger. Without a persistent local Preserves control surface, status/stop commands are direct CLI calls rather than requests crossing the node boundary.

## What Changes

- Add a local file/socket-style Preserves control profile with durable inbox and outbox artifacts under the explicit node state root.
- Persist canonical `node-control-request-v1` requests and receipt-backed queue/dispatch decisions.
- Add a node process/control lock artifact that binds the active startup receipt and rejects duplicate or stale control dispatch.
- Dispatch status and shutdown through the same submitted request path; fail closed for unsupported install/run/gate adapter operations before side effects.
- Import control requests, queue receipts, health/shutdown/suboperation receipts, and control receipts into the node ledger.

## Impact

Operators get a stable local control boundary that can later be backed by a Unix socket or stdio server without changing request/receipt semantics. The implementation remains local-only and deterministic; distributed admission, live async serving, and adapter-specific install/run/gate execution stay separate follow-up work.
