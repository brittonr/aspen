## ADDED Requirements

### Requirement: Executable VM fault matrix binds real fault evidence
r[molten.testing.executable_vm_fault_expansion.real_fault_matrix] Molten MUST represent executable VM fault coverage in a canonical fault support matrix that binds each fault kind, required capability, preflight refs, injection refs, child receipt refs, post-fault refs, expected outcome, actual decision, replay status, cleanup evidence, diagnostics, and topology-scoped caveats.

#### Scenario: Executable network fault records preflight and cleanup
- GIVEN a VM topology whose host supports network-control fault injection
- WHEN the harness injects a delay, drop, partition, rejoin, or asymmetric-latency fault
- THEN the fault receipt binds preflight, injection, child observation, post-fault, cleanup, and decision refs
- AND the support matrix records the fault as executable VM evidence for that topology only.

#### Scenario: Authority and corrupted receipt faults deny before side effects
- GIVEN stale ticket, wrong authority, conflicting operation id, or corrupted receipt inputs
- WHEN the VM fault harness applies the case
- THEN canonical denial receipts are emitted before side effects
- AND logs remain diagnostic-only attachments.

### Requirement: Unsupported or simulated VM faults are not pass evidence
r[molten.testing.executable_vm_fault_expansion.unavailable_policy] Molten MUST deny or mark diagnostic-only any VM fault claim whose required host support is unavailable, whose cleanup evidence is missing, whose observation is simulated-only, or whose evidence is logs without canonical fault receipts.

#### Scenario: Network-control unavailable does not mint pass evidence
- GIVEN a VM image without required network-control support
- WHEN the fault matrix requests a network partition claim
- THEN the harness emits unavailable or deny evidence according to policy
- AND the matrix does not count the case as executable pass evidence.

#### Scenario: Simulated fault remains diagnostic unless explicitly gated
- GIVEN a simulated fault case with no executable VM injection refs
- WHEN release-review fault coverage is evaluated
- THEN the case remains diagnostic-only
- AND it cannot satisfy executable VM fault coverage without a separate scope-accepting gate.
