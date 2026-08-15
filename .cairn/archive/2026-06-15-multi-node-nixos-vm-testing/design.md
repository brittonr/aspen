## Context

Molten already has local node daemon tests, live node-control workflow evidence, remote dataspace receipts, job worker execution, coordination services, delivery idempotency, source-gate validation, and operator dogfood. Those tests mostly run in one host process/tree or use local loopback fixtures. NixOS VM testing should prove that the same contracts survive the platform shell: Nix-built binaries, systemd units, VM networking, persisted state roots, reboot/restart, and cross-machine artifact transfer.

## Design

### NixOS test topology

Define a `testers.runNixOSTest` scenario with a minimal headless topology:

- `node-a` and `node-b`, each running the current Molten package from the same flake source and lock inputs;
- explicit per-node state roots under `/var/lib/molten` or a test-owned equivalent;
- a systemd service for `molten node run-loop` plus explicit init/run/status/stop commands where the test needs direct control;
- a driver-only staging area for generated source-gate, provenance, peer-ticket, authority, and workflow artifacts;
- VM networking by NixOS test node names, with no undeclared host network dependency.

The first implementation can stay small, but it must be production-shaped: real NixOS nodes, real systemd services, real filesystem state roots, and real cross-node request transfer through the documented CLI surfaces.

### Cross-node workflow

The happy-path VM workflow should:

1. build or install the current Molten package into both VMs;
2. initialize both node identities and state roots;
3. start node control loops and collect startup/health receipts;
4. exchange or import live peer-ticket and authority evidence;
5. submit a node-control workflow bundle from `node-a` to `node-b` and run apply/reconcile/ack/protocol-gate checks;
6. exercise at least one remote dataspace or service exchange;
7. run one job worker handoff or loopback execution path across the nodes;
8. run one coordination operation with operation-id/idempotency evidence;
9. export node-local evidence and collect canonical receipts from both VMs.

### Restart and durability

A second scenario or phase should stop/restart one VM while control work is queued or partially dispatched. The expected result is either deterministic idempotent completion after restart or a fail-closed recovery denial. The receipt should bind active-lock, inbox/outbox, ledger-readback, startup/shutdown, and recovery diagnostics.

### Receipt model

Add canonical VM-level evidence rather than treating terminal output as authoritative:

- `nixos-vm-topology-v1` binds node names, VM config refs, Nix input refs, Molten package/store refs, and network assumptions.
- `nixos-vm-node-evidence-v1` binds each node's startup, health, control-loop, shutdown, and exported-ledger refs.
- `nixos-vm-test-run-v1` binds topology refs, scenario/fault profile refs, child workflow refs, per-node evidence refs, replay status, diagnostics, log refs, result status, and evidence-only caveats.

VM tests are platform integration evidence. They are not deterministic runtime proof unless the relevant live observations are recorded and replayed through existing replay rails. Non-replayable VM observations must be explicitly marked and excluded from deterministic pass-evidence claims.

### Nix and CI surface

Expose the VM test as an explicit Nix check/app, for example `checks.<system>.nixos-vm-multinode` and/or an app for interactive debugging. The check must be headless and must not silently skip into a pass when KVM or VM execution support is unavailable. If a CI environment cannot run the VM test, that limitation should produce a diagnostic or keep the check outside the default fast gate rather than minting pass evidence.

### Non-goals

- Do not replace deterministic local harness, replay, Octet, Cairn, or dogfood gates.
- Do not treat VM evidence as authority, policy, provenance, resource, retention, or source-gate trust.
- Do not claim broad production network correctness beyond the declared VM topology and scenarios.
- Do not require internet access from the VMs beyond declared Nix build inputs and caches.
