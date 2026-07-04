## ADDED Requirements

### Requirement: Executable VM fault support matrix is explicit
r[molten.testing.nixos_vm.executable_fault_support_matrix] Molten SHOULD produce an executable VM fault support matrix that declares each fault kind, required capability, target node or link, command profile, expected outcome, host-support status, preflight refs, injection refs, child workflow refs, post-fault refs, diagnostics, diagnostic log refs, and caveats.

#### Scenario: Supported fault binds executable evidence
- GIVEN a VM fault descriptor whose required host or VM capability is available
- WHEN the VM injects the fault and validates the result
- THEN the receipt binds supported host status, pre-fault refs, injection refs, required child workflow refs, post-fault refs, diagnostics, and caveats
- AND the support matrix identifies the fault as executable evidence for the tested topology.

#### Scenario: Unsupported fault records unavailable evidence
- GIVEN a VM fault descriptor whose required host or VM capability is unavailable
- WHEN the VM fault check runs
- THEN the receipt records unavailable host support and diagnostic evidence
- AND unavailable execution does not satisfy pass evidence for that fault claim.

### Requirement: Executable VM fault validation rejects invalid claims
r[molten.testing.nixos_vm.executable_fault_validation_negatives] Molten MUST reject VM fault receipts that claim pass evidence without supported host status, required injection refs, required child workflow refs, matching topology, and canonical diagnostic evidence for denial or unavailable outcomes.

#### Scenario: Unsupported pass claim is rejected
- GIVEN a VM fault receipt that marks host support unavailable but claims a pass decision
- WHEN `fault-validate` evaluates the receipt
- THEN validation denies the receipt as an unsupported pass claim.

#### Scenario: Log-only pass claim is rejected
- GIVEN a VM fault descriptor and diagnostic log without the required canonical injection and child workflow refs
- WHEN `fault-validate` evaluates the evidence
- THEN validation denies before accepting the log as pass evidence.
