# VM runtime-host Phase 3 evidence

Captured: 2026-05-13T02:55:19Z

Raw logs are kept under ignored `target/runtime-proof/`; this committed summary omits cluster tickets and other secret material.

## Package gate

Command:

```bash
set -o pipefail; nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L 2>&1 | tee target/runtime-proof/aspen-node-vm-test-phase3-package-gate.log
```

Result: exit 0.

Classification: VM runtime-host node package closure builds successfully on the current host. This proves the package gate needed before nested-KVM E2E, but not guest execution by itself.

## E2E repair and rerun

The initial `vm-snapshot-e2e-test` attempt failed before product runtime assertions through a sandbox-disabled Nix/host-global `/tmp` interaction in the `bins.aspen-cli` / unit2nix build-plan path. The check wiring now uses the pure `aspen-cli-vm-test` package for this VM test.

The E2E test also submitted a `ci_vm` job with a local-executor payload. The snapshot worker registered with VM/runtime-host capabilities, but polling did not include the `local_executor` job type. The repair makes worker-only mode advertise and poll `local_executor` and aligns the VM test payload job type with that executor.

Verification before rerun:

```bash
git diff --check
cargo check --features node-runtime-apps,blob,automerge,ci,ci-vm-executor --bin aspen-node
nix eval --impure .#checks.x86_64-linux.vm-snapshot-e2e-test.drvPath >/dev/null
```

Result: exit 0.

Command:

```bash
set -o pipefail; nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false 2>&1 | tee target/runtime-proof/vm-snapshot-e2e-phase3-local-executor.log
```

Result: exit 0.

Proof markers from the passing E2E log:

```text
Snapshot VM worker registered with cluster: ASPEN_CI_NET_CONFIG ip=10.200.0.10 dev=eth0
CI job submitted
CI job completed via snapshot-restored VM
Second job (snapshot-restored VM): 1.4s
All stress test jobs completed
test script finished in 229.00s
```

Classification: reached final VM runtime-host E2E receipt boundary for the snapshot worker path: package build, cluster readiness, Cloud Hypervisor/snapshot worker boot, guest readiness marker, worker registration, job completion, second-job restore timing, stress-job completion, and test-driver cleanup all completed. Log lines from short-lived CLI processes include `Endpoint dropped without calling Endpoint::close`; these did not fail the test and are cleanup/noisy shutdown diagnostics rather than the proof boundary.

## Remaining Phase 3 scope

Hermit/uHyve and Hyperlight ignored product proofs are still pending under later Phase 3 tasks and are not claimed by this evidence file.
