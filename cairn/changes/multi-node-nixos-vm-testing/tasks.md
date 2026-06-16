## Phase 1: NixOS VM topology

- [x] [serial] r[molten.testing.nixos_vm_multinode.topology] Define a headless `testers.runNixOSTest` topology with at least two Molten NixOS nodes, explicit VM networking, current flake/package inputs, isolated state roots, and no undeclared host state.
- [x] [serial] r[molten.testing.nixos_vm_multinode.node_service] Package and run the real Molten node daemon/control loop under systemd in each VM with persistent identity, startup, health, and shutdown receipts.

## Phase 2: Cross-node workflows

- [ ] [serial] r[molten.testing.nixos_vm_multinode.control_workflow] Exercise cross-node live node-control workflow bundle handoff, apply/reconcile/ack, and protocol-gate evidence between VM nodes.
- [ ] [parallel] r[molten.testing.nixos_vm_multinode.service_job_coordination] Exercise at least one remote dataspace or service exchange, one job worker path, and one coordination operation across the VM nodes.
- [x] [parallel] r[molten.testing.nixos_vm_multinode.restart_durability] Add a restart/durability scenario for queued control work, ledger readback, active locks, and idempotent or fail-closed recovery.

## Phase 3: Evidence and CI surface

- [ ] [serial] r[molten.testing.nixos_vm_multinode.receipts] Emit canonical VM topology, per-node evidence, and VM test run receipts that bind Nix inputs/store refs, child receipts, replay status, diagnostics, logs, and evidence-only caveats.
- [x] [serial] r[molten.testing.nixos_vm_multinode.ci_gate] Expose the VM test through an explicit Nix check or app with headless configuration, KVM/CI diagnostics, and no silent skip-as-pass behavior.
