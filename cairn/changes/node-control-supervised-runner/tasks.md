# Tasks: Node Control Supervised Runner

## Phase 1: Canonical service artifacts

- [x] [serial] r[molten.node_control_service.spec.canonical_receipts] Add service lock, heartbeat, and run receipt schemas and ledger classification.
- [x] [serial] r[molten.node_control_service.spec.duplicate_lock] Add fail-closed active service lock acquisition and cleanup.

## Phase 2: Serve loop behavior

- [x] [serial] r[molten.node_control_service.spec.ingress_to_inbox] Scan local-Iroh ingress envelopes in deterministic order and deliver them before inbox drain.
- [x] [serial] r[molten.node_control_service.spec.loop_reuse] Reuse the existing bounded control loop for dispatch without bypassing operation gates.
- [x] [serial] r[molten.node_control_service.spec.shutdown_stop] Stop when shutdown dispatch removes the active control lock.

## Phase 3: CLI, coverage, validation

- [x] [parallel] r[molten.node_control_service.spec.canonical_receipts] Add `molten node serve` CLI with bounded tick/request controls and optional receipt output.
- [x] [parallel] r[molten.node_control_service.spec.tests] Cover duplicate runner denial, ingress-to-dispatch, shutdown stop, and heartbeat continuity.
- [x] [serial] r[molten.node_control_service.spec.tests] Run Molten validation gates and Cairn strict validation with the checked-out Cairn policy.
