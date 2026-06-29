## Phase 1: Diagnostics core

- [x] [serial] r[molten.node_runtime.network_diagnostics_report] Add pure diagnostics report types for NAT class, UDP reachability, relay latency, direct/relay status, interface/route refs, diagnostics, and caveats.
- [x] [serial] r[molten.node_runtime.connectivity_probe_receipts] Add pure connect/accept probe decisions and canonical connectivity probe receipts.

## Phase 2: Network watcher and port mapping

- [x] [serial] r[molten.node_runtime.network_watcher_snapshot] Add latest-state watcher snapshots for interface, address, route, relay, and endpoint status without unbounded event buffering.
- [x] [serial] r[molten.node_runtime.port_mapping_policy] Add deny-by-default port-mapping decisions for probe and mutate paths with explicit authority, policy, resource, scope, and duration evidence.

## Phase 3: Metrics evidence

- [x] [serial] r[molten.node_runtime.metrics_snapshot] Add bounded metrics snapshot types for node-control, live transport, queue depth, delivery idempotency, sync, and resource pressure counters.
- [x] [serial] r[molten.node_runtime.metrics_snapshot] Add OpenMetrics-style rendering or import tests with bounded, redacted label validation.

## Phase 4: Optional external diagnostics bridge

- [x] [serial] r[molten.node_runtime.external_diagnostics_bridge] Add optional iroh-services-style bridge decisions for metric push and remote diagnostics capability grants, with secret redaction and explicit operator config.
- [x] [serial] r[molten.node_runtime.external_diagnostics_bridge] Add negative tests for missing external-service policy, stale capability grants, secret leakage attempts, and unsupported remote diagnostics requests.

## Phase 5: CLI, VM, and soak integration

- [x] [serial] r[molten.testing.nixos_vm_multinode.network_diagnostics] Add CLI fixtures or node commands that emit local diagnostics, metrics snapshots, watcher snapshots, and probe receipts.
- [x] [serial] r[molten.testing.nixos_vm_multinode.network_diagnostics] Bind diagnostics and metrics refs into the NixOS multi-node VM test when available.
- [x] [serial] r[molten.prod_soak.network_diagnostics_observability] Bind network diagnostics, metrics snapshots, watcher state, relay latency, and resource-pressure refs into production-soak receipts.

## Phase 6: Documentation and validation

- [x] [serial] r[molten.node_runtime.network_diagnostics_report] Document the diagnostic/evidence-only trust boundary and the n0 reference projects.
- [x] [serial] r[molten.node_runtime.network_diagnostics_report] Run focused Rust tests, Cairn validation, and the smallest available VM/soak validation gate.
