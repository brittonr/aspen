# Runtime Host Readiness

This page records Aspen's current runtime-host evidence boundary for operator and reviewer use. It is a historical acceptance artifact, not a live status endpoint: rerun the gated checks at the commit you want to cite before making a new release/readiness claim.

## Current proven boundary

Aspen has guest-backed microVM CI execution evidence for the archived OpenSpec runtime-host matrix row:

```text
OpenSpec change: openspec/changes/archive/2026-05-07-add-runtime-host-e2e-matrix
matrix row: runtime-host-microvm-ci-vm
target: checks.x86_64-linux.vm-snapshot-e2e-test
proof level: aspen-spawned-execution
```

The proof is stronger than inventory registration. The gated check boots an Aspen node, creates a Cloud Hypervisor golden snapshot, restores guest VMs from that snapshot, registers a guest worker with the Aspen cluster, submits real `ci_vm` jobs through `aspen-cli`, and waits for job completion.

## Latest accepted evidence

Latest accepted runtime-host microVM evidence on `main`:

```text
commit: 860a1d6c8 Trim snapshot VM E2E diagnostics
implementation commit: 56006f9cc Prove snapshot VM CI execution
gated check: nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false
result: passed
derivation: /nix/store/x5yz9rni6c269sq4lrc0ka5fzdjfx7zv-vm-test-run-vm-snapshot-e2e.drv
evidence log: .agent/evidence/runtime-host-e2e/vm-snapshot-e2e-trimmed-pass-20260508.log
log length: 928 lines
duration: test script finished in 96.45s
```

The earlier full proof log from the implementation commit remains useful for forensic comparison:

```text
evidence log: .agent/evidence/runtime-host-e2e/vm-snapshot-e2e-pass-20260508.log
log length: 1398 lines
derivation: /nix/store/3dd19akflhxgvp4rz2yf2zfkzi0m1yy9-vm-test-run-vm-snapshot-e2e.drv
```

## Proof markers to require

A passing check is only runtime-host evidence when the log includes these proof markers:

- Guest network configuration from the direct-boot microVM path, for example `ASPEN_CI_NET_CONFIG ip=10.200.0.10 dev=eth0`.
- Guest worker registration, for example `worker registered with cluster` in the guest serial log and `Snapshot VM worker registered with cluster` from the host test script.
- Real job execution through Aspen CI, specifically `CI job completed via snapshot-restored VM`.
- Snapshot restore reuse evidence, such as `Second job (snapshot-restored VM): <seconds>s`.
- Concurrent restored-VM stress evidence, including `All stress test jobs completed` and the COW efficiency summary.

Do not treat any of the following as sufficient by themselves:

- the OpenSpec archive existing;
- the test-harness inventory listing the row;
- Cloud Hypervisor snapshot files existing;
- an Aspen node starting successfully;
- a package build of `aspen-node-vm-test` without the gated VM check.

## How to reproduce

The check requires nested KVM and is intentionally not part of cheap/default verification. Run the package gate first so packaging failures do not consume a full VM-test cycle:

```bash
nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L
nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false
```

If only refreshing the derivation path or checking Nix evaluation:

```bash
nix eval --impure .#checks.x86_64-linux.vm-snapshot-e2e-test.drvPath --raw
```

## What the check covers

The accepted `runtime-host-microvm-ci-vm` path currently covers:

1. host-side Aspen cluster/node startup with worker and CI support enabled;
2. VM executor access to the configured Cloud Hypervisor, VirtioFS, kernel, initrd, toplevel, and `ip` binary;
3. host-created TAP attachment to the Aspen CI bridge;
4. direct-boot guest network bootstrap on `eth0` with route to the host bridge;
5. golden snapshot creation with Cloud Hypervisor `memory-ranges` artifacts and Aspen `ticket.txt`;
6. snapshot-restored guest worker registration with the cluster;
7. `ci_vm` job submission using the local-executor payload schema (`command`, `args`, `timeout_secs`);
8. job completion through the restored guest worker;
9. a second job for snapshot-restore latency evidence;
10. an eight-job stress slice that forces concurrent restores and records COW efficiency.

## Boundaries and caveats

- This is a gated, impure nested-KVM acceptance check. It is expected to be run deliberately, not as a default local smoke test.
- The current proven host class is Cloud Hypervisor microVM CI. Metadata-only matrix rows for other host classes are visibility gaps until promoted to runnable checks with receipts.
- The verified E2E path uses `microvm.postBootCommands` to emit readiness markers, configure guest networking, and launch the worker in the direct-boot image. Do not promote direct systemd target boot to a product guarantee without a separate design/spec and fresh E2E evidence.
- The check uses `/tmp` VM state inside the NixOS test guest and removes it during cleanup. Preserve separate `.agent/evidence/...` logs when citing historical evidence.
- Logs and incident notes must redact cluster tickets, `aspen://...` remotes, bearer values, private keys, connection strings, and private checkout/source URLs as `[REDACTED]`.

## Operator checklist

Before citing runtime-host readiness:

- Confirm the commit under review is at or after the implementation commit you intend to cite.
- Run or cite the package gate and gated nested-KVM check command.
- Confirm the log has the proof markers above, not just inventory registration or snapshot artifacts.
- Record the derivation path and evidence log path.
- State the host class precisely: `runtime-host-microvm-ci-vm` / Cloud Hypervisor microVM CI.
- State unsupported or metadata-only host classes separately.
- Redact secrets before copying any log excerpts.
