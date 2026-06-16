## ADDED Requirements

### Requirement: NixOS multi-node VM topology
r[molten.testing.nixos_vm_multinode.topology] Molten MUST provide a NixOS VM integration test topology, implemented with `testers.runNixOSTest` or an equivalent NixOS test driver, that starts at least two Molten nodes with explicit VM networking, headless configuration, current flake/package inputs, isolated state roots, and no undeclared host state.

#### Scenario: VM topology starts two isolated Molten nodes
- GIVEN the current Molten source tree and Nix inputs
- WHEN the NixOS VM integration test topology is built
- THEN it defines at least two headless NixOS nodes with Molten installed from the same package derivation
- AND each node has an explicit state root, persistent identity location, and declared VM network identity.

### Requirement: Molten node service runs inside each VM
r[molten.testing.nixos_vm_multinode.node_service] Molten MUST run the real Molten node daemon or control loop under systemd inside each VM node, with startup, health, control-loop, shutdown, persistent-identity, and state-root evidence collected as canonical receipts.

#### Scenario: VM node readiness is receipt backed
- GIVEN a VM node configured for Molten
- WHEN the node service reaches ready state
- THEN the test collects startup and health receipt refs for the configured state root and persistent identity
- AND shutdown or restart collects matching node shutdown or recovery evidence.

### Requirement: Cross-node node-control workflow coverage
r[molten.testing.nixos_vm_multinode.control_workflow] Molten MUST exercise cross-node node-control workflow bundle handoff between VM nodes, including peer-ticket or endpoint evidence, authority evidence, bundle apply, reconcile, ack, and protocol-gate receipts.

#### Scenario: Bundle handoff crosses the VM network
- GIVEN two VM nodes with admitted peer and authority evidence
- WHEN `node-a` sends or stages a node-control workflow bundle for `node-b`
- THEN `node-b` applies or denies the bundle through the same control inbox and control-loop path used by the node daemon
- AND the final evidence binds apply, reconcile, ack, protocol-gate, ingress, queue, and control receipt refs.

### Requirement: Cross-node service, job, and coordination paths
r[molten.testing.nixos_vm_multinode.service_job_coordination] Molten SHOULD exercise at least one remote dataspace or service exchange, one job worker handoff or execution path, and one coordination operation across the VM nodes, binding each child receipt into the VM test run evidence.

#### Scenario: VM test binds distributed child receipts
- GIVEN a passing multi-node VM run
- WHEN the test run receipt is emitted
- THEN it includes child refs for a remote dataspace or service exchange, a job worker path, and a coordination operation
- AND each child receipt preserves its normal authority, policy, resource, provenance, source-gate, and retention checks separately.

### Requirement: Restart and durability VM scenario
r[molten.testing.nixos_vm_multinode.restart_durability] Molten MUST include a VM scenario that restarts or stops a node while control work is queued or partially dispatched, then verifies ledger readback, active-lock handling, queued request idempotency, and fail-closed recovery diagnostics.

#### Scenario: Restart handles queued control work deterministically
- GIVEN a control request is queued for a VM node
- WHEN the node is restarted before the request is fully dispatched
- THEN the resumed node either completes the request idempotently with matching receipt refs or emits a recovery denial before side effects
- AND the VM test evidence binds active-lock, inbox, outbox, ledger-readback, startup, shutdown, and recovery diagnostics.

### Requirement: Canonical NixOS VM test receipts
r[molten.testing.nixos_vm_multinode.receipts] Molten MUST emit canonical VM-level receipts for NixOS VM tests, including topology refs, node evidence refs, Nix input or store refs, scenario and fault-profile refs, child workflow refs, replay status, diagnostics, log refs, decision status, and explicit evidence-only caveats.

#### Scenario: Terminal output is not authoritative VM evidence
- GIVEN a VM integration test completes
- WHEN the result is evaluated by CI, release, or operator workflows
- THEN pass or deny status is read from canonical `nixos-vm-test-run-v1` or equivalent receipt evidence
- AND raw terminal output, QEMU logs, and systemd journals are bound as diagnostic refs rather than treated as authoritative pass evidence.

### Requirement: Explicit Nix/CI VM gate surface
r[molten.testing.nixos_vm_multinode.ci_gate] Molten SHOULD expose the multi-node VM test through an explicit Nix check or app with headless configuration and documented KVM/CI requirements. The gate MUST NOT silently convert skipped or unsupported VM execution into passing evidence.

#### Scenario: Missing VM support does not mint pass evidence
- GIVEN a CI environment without the required VM execution support
- WHEN the multi-node NixOS VM test check is requested
- THEN Molten emits a diagnostic failure, skip receipt, or unavailable status that is not accepted as pass evidence
- AND any default fast validation gate documents whether the VM test was executed or intentionally excluded.
