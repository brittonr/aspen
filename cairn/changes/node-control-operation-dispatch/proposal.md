## Why

The persistent node control profile now durably submits and dispatches Preserves requests, but adapter-style operations still stop at fail-closed placeholders. Operators need the same control boundary to perform the first useful local node actions: install an artifact into the node registry, run an admitted job against node-local state, and validate gate evidence for a target artifact.

## What Changes

- Wire `install`, `run`, and `gate` control operations to existing Molten artifact registry, job execution, and Octet source-gate cores.
- Require operation payload and target refs to resolve from the node ledger before side effects.
- Preserve fail-closed behavior for missing authority, policy, resource, payload, target, stale lock, denied suboperations, or tampered evidence.
- Import operation subreceipts and final control receipts into the node ledger.

## Impact

The node control inbox becomes a usable local control plane while staying file-backed and deterministic. This does not add a long-running socket server or distributed admission protocol; it only routes already canonical requests through receipt-backed local operations.
