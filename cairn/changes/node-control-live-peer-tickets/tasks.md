# Tasks: Node Control Live Peer Tickets

## Phase 1: Ticket/admission evidence

- [x] [serial] r[molten.node_control_live_peer_tickets.spec.ticket_artifacts] Add canonical live ticket artifacts and ledger classification.
- [x] [serial] r[molten.node_control_live_peer_tickets.spec.peer_admission_artifacts] Add canonical live peer admission receipts and ledger classification.

## Phase 2: CLI workflow

- [x] [parallel] r[molten.node_control_live_peer_tickets.spec.ticket_cli] Add `molten node live-ticket-export` and `serve --live-ticket-out`.
- [x] [parallel] r[molten.node_control_live_peer_tickets.spec.peer_admit_cli] Add `molten node live-peer-admit`.

## Phase 3: Live ingress gate

- [x] [serial] r[molten.node_control_live_peer_tickets.spec.live_pre_enqueue_gate] Resolve live peer bootstrap refs to admitted ticket evidence before enqueue.
- [x] [serial] r[molten.node_control_live_peer_tickets.spec.transport_non_authority] Keep tickets/admissions separate from authority and provenance gates.
- [x] [parallel] r[molten.node_control_live_peer_tickets.spec.fail_closed] Add unit and CLI coverage for admitted and denied live peer bootstrap paths.
