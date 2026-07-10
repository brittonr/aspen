## ADDED Requirements

### Requirement: Executable VM network faults use declared backends
r[molten.testing.nixos_vm_fault_injection.executable_network_backend] Molten SHOULD execute bounded VM network delay, drop, partition, rejoin, and asymmetric-latency faults when a declared network-control backend is supported, and it MUST record unavailable or deny evidence without minting pass evidence when support is absent.

#### Scenario: Supported backend executes a network partition
- GIVEN a VM topology link and a network-control probe whose backend is supported
- WHEN a bounded partition fault is injected, observed, removed, and validated
- THEN the fault receipt binds backend support, topology ref, preflight refs, injection refs, required child workflow refs, cleanup refs, post-fault refs, diagnostics, and caveats
- AND the support matrix identifies the case as executable evidence for that topology.

#### Scenario: Missing backend remains unavailable
- GIVEN a VM image or host without a supported network-control backend
- WHEN a network fault is requested
- THEN the capability and fault receipts record unavailable support
- AND the fault matrix does not count the case as pass evidence.

### Requirement: VM network fault cleanup is mandatory for pass evidence
r[molten.testing.nixos_vm_fault_injection.network_fault_cleanup_validation] Molten MUST reject network-fault pass claims when cleanup evidence, post-fault checks, required child workflow refs, matching topology, or canonical diagnostics are missing.

#### Scenario: Missing cleanup denies pass
- GIVEN a network fault receipt with injection evidence but no cleanup or post-fault evidence
- WHEN fault validation evaluates the receipt
- THEN validation denies before pass evidence is accepted
- AND diagnostics identify the missing cleanup boundary.

#### Scenario: Log-only network fault claim is rejected
- GIVEN diagnostic logs that appear to show a partition and rejoin but omit canonical injection, child workflow, cleanup, or post-fault refs
- WHEN fault validation evaluates the evidence
- THEN the pass claim is rejected before logs are considered authoritative.
