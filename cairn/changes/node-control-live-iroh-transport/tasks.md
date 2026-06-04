# Tasks: Node Control Live Iroh Transport

## Phase 1: Canonical live transport

- [x] [serial] r[molten.node_control_live_iroh.spec.transport_receipts] Add live transport receipt schema and ledger classification.
- [x] [serial] r[molten.node_control_live_iroh.spec.gossip_bytes] Add live `iroh-gossip` envelope construction, canonical-byte publish, and canonical-byte receive helpers.

## Phase 2: Durable ingress boundary

- [x] [serial] r[molten.node_control_live_iroh.spec.durable_ingress] Store live-received envelopes in the existing ingress area and call existing ingress delivery.
- [x] [serial] r[molten.node_control_live_iroh.spec.transport_not_authority] Preserve existing authority, resource, idempotency, provenance, and source-gate enforcement after live receive.

## Phase 3: CLI, loopback, validation

- [x] [parallel] r[molten.node_control_live_iroh.spec.loopback_tests] Add local two-endpoint Iroh gossip loopback coverage and CLI command.
- [x] [serial] r[molten.node_control_live_iroh.spec.loopback_tests] Run Molten validation gates and Cairn strict validation with the checked-out Cairn policy.
