## ADDED Requirements

### Requirement: VM network-control capability is probed before network faults
r[molten.testing.nixos_vm_fault_injection.network_control_probe] Molten MUST record explicit VM network-control capability evidence before claiming executable network delay, drop, partition, rejoin, or asymmetric latency fault coverage.

#### Scenario: Supported network-control backend is recorded
- GIVEN a VM image and test-driver environment with a supported network-control backend
- WHEN the network fault preflight runs
- THEN the capability receipt binds the backend, target link, topology ref, cleanup strategy, and supported host status
- AND the executable fault may proceed only through that declared backend.

#### Scenario: Missing network-control backend remains unavailable
- GIVEN a VM image or host without a supported network-control backend
- WHEN a network fault is requested
- THEN the capability receipt records unavailable support
- AND the fault matrix does not count the case as pass evidence.

### Requirement: Executable VM network faults bind injection and cleanup evidence
r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Molten SHOULD execute bounded VM network faults on capable hosts and MUST bind injection evidence, child workflow refs, cleanup evidence, topology refs, diagnostics, and caveats before accepting pass evidence.

#### Scenario: Partition and rejoin produce canonical evidence
- GIVEN a supported VM topology link and a declared cross-node workflow
- WHEN a partition fault is injected, observed, and removed
- THEN the fault receipt binds preflight, injection, child workflow, cleanup, and post-fault refs
- AND the resulting decision reflects idempotent recovery or deny-before-side-effects evidence.

#### Scenario: Missing cleanup denies pass
- GIVEN a network fault run with injection evidence but no cleanup evidence
- WHEN fault validation evaluates the receipt
- THEN validation denies before pass evidence is accepted
- AND diagnostics identify the missing cleanup boundary.
