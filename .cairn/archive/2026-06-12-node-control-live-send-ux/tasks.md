# Tasks: Node Control Live Send UX

## Phase 1: Send evidence

- [x] [serial] r[molten.node_control_live_send_ux.spec.send_receipt] Add canonical live send receipts and ledger classification.
- [x] [serial] r[molten.node_control_live_send_ux.spec.workflow_receipt] Add canonical live workflow runbook receipts and ledger classification.
- [x] [serial] r[molten.node_control_live_send_ux.spec.ticket_endpoint_binding] Bind send receipts to receiver ticket endpoint/address evidence.

## Phase 2: CLI workflow

- [x] [serial] r[molten.node_control_live_send_ux.spec.live_send_cli] Add `molten node control-ingress-live-send` with send/transport receipt outputs.
- [x] [serial] r[molten.node_control_live_send_ux.spec.workflow_cli] Add `molten node live-workflow-bundle` for operator runbook receipts.

## Phase 3: Real live path and gates

- [x] [serial] r[molten.node_control_live_send_ux.spec.real_gossip_send] Join the receiver's real Iroh gossip topic from ticket evidence and publish canonical envelope bytes.
- [x] [parallel] r[molten.node_control_live_send_ux.spec.transport_non_authority] Keep live send evidence separate from authority and provenance gates.
- [x] [parallel] r[molten.node_control_live_send_ux.spec.fail_closed] Add unit and CLI coverage for successful bounded send/listen and offline-ticket denial.
