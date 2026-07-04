## ADDED Requirements

### Requirement: VM fault descriptors are canonical
r[molten.testing.nixos_vm_fault_injection.fault_descriptors] Molten MUST define canonical VM fault descriptors for executable NixOS VM fault cases, including target node or link, fault kind, command profile, expected outcome, bounded duration or trigger condition, preflight refs, and evidence-only caveats.

#### Scenario: Fault descriptor binds target and expectation
- GIVEN a VM network partition fault targeting traffic from one node to another
- WHEN Molten canonicalizes the fault descriptor
- THEN the descriptor binds the source node, target node, fault kind, expected recovery or denial outcome, and preflight evidence refs.

### Requirement: Unsupported VM fault execution does not pass silently
r[molten.testing.nixos_vm_fault_injection.unavailable_boundary] Executable VM fault checks MUST fail closed or emit unavailable evidence when required KVM, QEMU, test-driver, network-control, filesystem, or privilege support is missing. Unsupported executable faults MUST NOT be converted into passing distributed-test evidence.

#### Scenario: Missing network-control support is unavailable
- GIVEN a CI host or VM image cannot execute the requested network fault injection
- WHEN the VM fault check requests that case
- THEN Molten records unavailable or deny evidence for that fault
- AND the final VM fault matrix does not count the case as pass evidence.

### Requirement: VM network faults are executable where supported
r[molten.testing.nixos_vm_fault_injection.network_faults] Molten SHOULD execute representative network delay, drop, one-way partition, rejoin, and asymmetric latency faults inside the NixOS VM topology when host and VM support are available, and bind resulting child workflow evidence into the VM fault receipt.

#### Scenario: Partition and rejoin preserve safety
- GIVEN two VM nodes with queued node-control or service workflow evidence
- WHEN an executable partition fault is injected and later removed
- THEN the resulting receipts show either idempotent recovery with matching operation refs or deny-before-side-effects diagnostics.

### Requirement: VM restart windows are exercised
r[molten.testing.nixos_vm_fault_injection.restart_windows] Molten MUST exercise crash, stop, or restart windows around queued control work, partial dispatch, duplicate send, service heartbeat, and receipt write/readback paths in at least one executable VM fault check.

#### Scenario: Duplicate send after restart is idempotent
- GIVEN a sender VM has emitted a send receipt for an operation
- WHEN the sender restarts and attempts the same send again
- THEN the receiver evidence shows duplicate suppression or idempotent replay
- AND no second semantic commit is accepted for the same operation id.

### Requirement: VM storage and state-root faults are bounded
r[molten.testing.nixos_vm_fault_injection.storage_state_faults] Molten SHOULD execute bounded storage and state-root fault cases such as missing artifacts, permission-denied state roots, corrupted diagnostic-only logs, or bounded disk pressure where deterministic VM support permits.

#### Scenario: Permission-denied state root fails before mutation
- GIVEN a VM node state root is made unwritable for a targeted operation
- WHEN the operation attempts to write control, ledger, or receipt state
- THEN Molten emits a denial or failure receipt before accepting side effects as pass evidence.

### Requirement: VM executable faults emit canonical receipts
r[molten.testing.nixos_vm_fault_injection.fault_receipts] Molten MUST emit canonical VM fault receipts that bind fault descriptor refs, host-support status, pre-fault refs, injection evidence refs, post-fault child refs, decisions, diagnostics, replay status, diagnostic log refs, and evidence-only caveats.

#### Scenario: Fault receipt binds pre and post evidence
- GIVEN an executable VM fault case completes
- WHEN the VM fault receipt is emitted
- THEN it identifies the fault descriptor, preflight evidence, injection evidence, observed child receipts, final decision, and any unavailable or degraded diagnostics.

### Requirement: VM executable fault validation has negative fixtures
r[molten.testing.nixos_vm_fault_injection.negative_fixtures] Molten SHOULD test VM executable fault validation with negative fixtures for unsupported host support, stale evidence refs, tampered fault descriptors, wrong topology membership, missing child refs, and log-only pass claims.

#### Scenario: Log-only pass is rejected
- GIVEN a VM fault run whose logs claim success but whose canonical fault receipt is missing or denied
- WHEN validation evaluates the run
- THEN validation rejects pass evidence and treats logs as diagnostic-only.

### Requirement: VM executable fault docs define boundaries
r[molten.testing.nixos_vm_fault_injection.docs] User-facing documentation SHOULD describe how to run executable VM fault checks, required host support, unavailable handling, authoritative receipt paths, diagnostic logs, and the limits of VM platform evidence.

#### Scenario: Operator inspects fault evidence
- GIVEN a realized VM fault check output
- WHEN an operator follows the documentation
- THEN they can identify the canonical fault receipts, unsupported-case diagnostics, child workflow refs, and evidence-only caveats without relying on raw build logs.
