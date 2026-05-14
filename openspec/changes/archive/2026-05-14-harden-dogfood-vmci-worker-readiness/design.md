## Context

VM-CI mode intentionally skips the local `NixBuildWorker` so Cloud Hypervisor VMs handle `ci_nix_build` and `ci_vm` jobs. On a host that cannot create TAP devices, VM pool initialization can continue with zero idle VMs. Ordinary worker threads exclude VM-only job types, so the CI pipeline has no eligible executor and the dogfood command waits instead of failing at the actual readiness boundary.

## Design

### Readiness gate

When dogfood runs with `vm_ci=true`, Aspen should require a VM worker readiness signal before accepting the pipeline wait phase as meaningful. The signal can be one of:

- at least one pre-warmed VM in the pool;
- a registered VM-capable worker with positive capacity;
- an explicit readiness receipt from the VM pool manager.

If the VM pool reports zero capacity because TAP/TUN, KVM, Cloud Hypervisor, image, or workspace provisioning failed, the run should fail the relevant stage before or immediately after CI trigger.

### Failure receipt

The dogfood receipt should record a failed stage with:

- operation: `vm_ci_worker_readiness` or equivalent;
- category: VM worker readiness / host capability;
- message: redacted, bounded failure summary;
- artifacts or diagnostics pointing at local log paths and readiness counters.

### CI wait guard

The CI wait loop should also detect impossible scheduling for VM-only job types: a pending pipeline whose jobs require `ci_nix_build`/`ci_vm` while the cluster reports zero eligible workers should fail deterministically rather than waiting for timeout.

## Risks

- **False negatives during startup**: mitigate with a short bounded startup grace period and explicit transition from initializing to zero-capacity failed state.
- **Overfitting to TAP**: categorize TAP/TUN as one readiness cause but keep the gate generic for KVM/image/workspace failures.
- **Secret leakage**: receipts should include capability names and log handles, not tickets, cookies, or raw connection material.
