## Design

The control surface is modeled as a file-backed local Preserves transport:

- `control/inbox/` contains submitted canonical `node-control-request-v1` values named by request ref.
- `control/outbox/` contains dispatched request copies and canonical control receipts named by request ref.
- `control/node.lock.preserves` records the active startup receipt and local state-root profile; it is not an authority token by itself.
- `node-control-queue-receipt-v1` records request submission/dispatch bookkeeping and binds the request ref.
- `node-control-operation-receipt-v1` records fail-closed adapter dispatch decisions for install/run/gate until those adapters are wired to real node operations.

Dispatch is single-request and deterministic. A submitted status request runs the existing health path but binds the submitted request ref in the resulting `node-control-receipt-v1`. A submitted shutdown request runs graceful shutdown and removes the active lock only after writing/importing shutdown and control receipts. Unsupported or unwired operations produce deny receipts before side effects.

The node ledger under `<state-root>/ledger` imports every canonical request and receipt value so catalog/MCP views can later inspect the control history through the existing ledger/catalog path. Ledger import receipts are written under `<state-root>/receipts` as derived evidence.

## Boundaries

This change does not start a long-running async socket server. The durable file/inbox profile is the normative control boundary; a Unix socket or stdio server can be a later imperative shell over the same records. Rendered CLI text remains non-normative.
