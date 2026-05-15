## MODIFIED Requirements

### Requirement: VM-CI Dogfood Worker Readiness [r[dogfood-evidence.vmci-worker-readiness]]

VM-CI dogfood readiness MUST distinguish direct TAP privilege requirements from helper-backed TAP lifecycle requirements, and MUST emit bounded receipt evidence when either boundary is unavailable.

#### Scenario: Direct TAP mode requires runtime network administration

- GIVEN VM-CI dogfood is configured with `ASPEN_CI_NETWORK_MODE=tap`
- AND the current process lacks `CAP_NET_ADMIN`
- WHEN the dogfood run performs VM-CI readiness checks
- THEN readiness fails before waiting on the CI pipeline
- AND the receipt failure category is `vm_ci_readiness`
- AND the diagnostic says direct TAP mode requires `CAP_NET_ADMIN` or `tap-helper` mode.

#### Scenario: TAP helper mode requires an executable helper

- GIVEN VM-CI dogfood is configured with `ASPEN_CI_NETWORK_MODE=tap-helper`
- AND `ASPEN_CI_TAP_HELPER_PATH` is missing or not executable
- WHEN the dogfood run performs VM-CI readiness checks
- THEN readiness fails before waiting on the CI pipeline
- AND the receipt failure category is `vm_ci_readiness`
- AND the diagnostic names the missing helper path requirement.

#### Scenario: Helper-backed TAP lifecycle stays allowlisted

- GIVEN VM-CI runtime is configured with `NetworkMode::TapWithHelper`
- WHEN a VM TAP device is prepared or cleaned up
- THEN the runtime invokes the configured helper instead of direct `ip` TAP mutation
- AND the helper only accepts `ci-n*-vm*-tap` device names and bridge `aspen-ci-br0`
- AND invalid device names, invalid bridges, or unknown actions are rejected before invoking `ip`.

#### Scenario: Dogfood defaults to installed helper

- GIVEN `setup-ci-network` has installed an executable TAP helper at `/usr/local/libexec/aspen-ci-tap-helper`
- AND the operator did not explicitly set `ASPEN_CI_NETWORK_MODE`
- WHEN `dogfood-local-vmci` starts
- THEN it selects `tap-helper` mode and exports `/usr/local/libexec/aspen-ci-tap-helper` as the helper path
- AND the default avoids `nosuid` temporary mounts where file capabilities can be ignored
- AND the `aspen-node` process does not need ambient `CAP_NET_ADMIN` for TAP lifecycle operations.
