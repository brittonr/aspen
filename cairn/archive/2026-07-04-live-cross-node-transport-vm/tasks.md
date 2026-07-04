# Tasks: live-cross-node-transport-vm

## Phase 1: VM scenario shape

- [x] [parallel] r[molten.testing.nixos_vm.cross_node_live_transport] Define the live cross-node transport scenario fixture and receipt binding model for sender, receiver, topic, operation id, ticket, admission, send, receive, ingress, queue, dispatch, reconcile, ack, and protocol gate.
- [x] [parallel] r[molten.testing.nixos_vm.live_transport_negative_gate] Define negative validation cases for wrong peer, wrong node, stale ticket, missing receive receipt, missing protocol gate, and log-only pass claims.

## Phase 2: VM execution and validation

- [x] [serial] r[molten.testing.nixos_vm.cross_node_live_transport] Extend the NixOS VM check so the request crosses the live transport boundary before test-driver artifact export.
- [x] [serial] r[molten.testing.nixos_vm.live_transport_negative_gate] Add receipt parser and gate tests proving stale or missing live transport evidence denies before VM pass evidence.

## Phase 3: Evidence readback

- [x] [parallel] r[molten.testing.nixos_vm.cross_node_live_transport] Update `docs/distributed-testing.md` or VM docs with live-transport evidence scope and inspection commands.
- [x] [serial] r[molten.testing.nixos_vm.live_transport_negative_gate] Run focused VM receipt tests and the smallest relevant VM check, or record host-support blockers and next best checks.
