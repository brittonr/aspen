## ADDED Requirements

### Requirement: Multi-node VM network diagnostics evidence
r[molten.testing.nixos_vm_multinode.network_diagnostics] Molten SHOULD bind local network diagnostics reports, connectivity probe receipts, route/interface watcher snapshots, and metrics snapshot refs into the NixOS multi-node VM test-run evidence when the host environment can execute those checks.

#### Scenario: VM run binds diagnostics child refs
- GIVEN a multi-node VM test completes network diagnostics and metrics snapshots for each node
- WHEN the VM test-run receipt is emitted
- THEN it includes child refs for diagnostics reports, connectivity probes, watcher snapshots, and metrics snapshots
- AND raw terminal logs remain diagnostic refs rather than authoritative pass evidence.

#### Scenario: Missing host support does not mint diagnostic pass evidence
- GIVEN the host cannot perform a required VM network diagnostic or port-map probe
- WHEN the VM check requests that diagnostic
- THEN Molten records unavailable or degraded diagnostics
- AND the VM check does not convert the unavailable diagnostic into pass evidence.
