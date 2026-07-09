# Tasks: live-topology-transport-profiles

## Phase 1: Profile models and preflight

- [ ] [serial] r[molten.peer_bootstrap.live_topology_profile] Define canonical live topology profile values and pure topology preflight checks.
- [ ] [serial] r[molten.peer_bootstrap.live_transport_profile] Define canonical transport profile values and admission under runtime hard caps.
- [ ] [serial] r[molten.peer_bootstrap.live_profiles_non_authority] Preserve independent authority, policy, resource, provenance, source-gate, retention, and capability gates in the preflight model.

## Phase 2: CLI/live workflow integration

- [ ] [parallel] r[molten.peer_bootstrap.live_profile_receipts] Bind selected topology and transport profile refs into live-send, listener, ticket, and workflow-bundle receipts.
- [ ] [parallel] r[molten.peer_bootstrap.live_topology_profile] Add optional profile inputs to representative node live commands while preserving explicit flags.
- [ ] [parallel] r[molten.peer_bootstrap.live_transport_profile] Thread admitted retry/timeout values into live-send/listener shells only after preflight.

## Phase 3: Tests and validation

- [ ] [parallel] r[molten.peer_bootstrap.live_topology_profile] Add positive tests for matching topology and negative tests for wrong peer, topic, endpoint, and ALPN.
- [ ] [parallel] r[molten.peer_bootstrap.live_transport_profile] Add positive tests for admitted retry/timeout values and negative over-cap tests.
- [ ] [parallel] r[molten.peer_bootstrap.live_profile_receipts] Add receipt tests for profile refs, explicit flag caveats, and effective values.
- [ ] [parallel] r[molten.peer_bootstrap.live_profiles_non_authority] Add negative tests for transport/topology profile use without authority or policy evidence.
- [ ] [serial] r[molten.peer_bootstrap.live_topology_profile] Run focused peer/node live tests and Cairn proposal/design/tasks/spec gates.
