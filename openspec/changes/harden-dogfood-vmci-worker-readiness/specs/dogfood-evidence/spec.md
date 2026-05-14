## ADDED Requirements

### Requirement: VM-CI Dogfood Worker Readiness [r[dogfood-evidence.vmci-worker-readiness]]
Aspen MUST fail VM-CI dogfood acceptance deterministically when VM-only CI jobs cannot be scheduled because no VM-capable worker is ready.

#### Scenario: VM-CI fails fast with zero VM capacity [r[dogfood-evidence.vmci-worker-readiness.zero-capacity]]
- GIVEN dogfood is running in VM-CI mode
- AND the local shell workers exclude VM-only job types such as `ci_nix_build` or `ci_vm`
- AND the Cloud Hypervisor VM pool reports zero ready capacity after a bounded startup grace period
- WHEN the dogfood run reaches the CI build wait phase
- THEN the run SHALL fail with a VM worker readiness failure instead of waiting indefinitely for an unschedulable pipeline
- AND the run SHALL NOT be reported as full dogfood acceptance

#### Scenario: TAP or TUN denial is diagnosable [r[dogfood-evidence.vmci-worker-readiness.tap-tun-denied]]
- GIVEN VM pool initialization fails because TAP/TUN device creation is denied by the host
- WHEN dogfood records the failure
- THEN the receipt or diagnosis SHALL include a redacted host-capability category and bounded message identifying TAP/TUN readiness
- AND it SHALL include local evidence handles without exposing tickets, cookies, private keys, or connection strings

#### Scenario: Successful VM-CI readiness is preserved [r[dogfood-evidence.vmci-worker-readiness.ready]]
- GIVEN dogfood is running in VM-CI mode on a host with VM-capable workers ready
- WHEN the pipeline contains VM-only CI jobs
- THEN the readiness gate SHALL allow the normal CI wait and receipt flow to continue
