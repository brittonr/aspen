## Why

A current-head `dogfood-local-vmci full` proof was attempted with `--cluster-dir /home/brittonr/data/aspen-dogfood-vmci`. The cluster started and pushed source, but the Cloud Hypervisor CI VM pool could not create TAP devices on this host:

```text
create CI VM TAP device: ip tuntap add dev ci-n1-vm1-tap mode tap failed: ioctl(TUNSETIFF): Operation not permitted
```

The run then continued with `idle_vm_count=0` and waited for the CI pipeline even though all ordinary workers explicitly excluded `ci_nix_build` and `ci_vm`. That leaves operators with a long wait and a partial receipt instead of an immediate, diagnosable VM-CI readiness failure.

## What Changes

- Require VM-CI dogfood mode to fail fast when no VM-capable CI worker is available for VM-only job types.
- Record a redacted receipt failure stage that identifies VM worker readiness, TAP/TUN capability, and worker-count evidence without leaking secrets.
- Preserve successful VM-CI behavior when at least one VM worker is available.

## In Scope

- Dogfood VM-CI readiness checks before or immediately after pipeline trigger.
- CI worker readiness reporting for zero VM pool capacity and TAP/TUN permission failures.
- Operator-facing diagnosis for partial VM-CI receipts.

## Out of Scope

- Changing host privilege setup or granting TAP/TUN permissions from Aspen itself.
- Replacing the existing successful local shell-worker dogfood path.
- Treating a partial VM-CI receipt as full dogfood acceptance.

## Evidence

- Attempt log: `target/runtime-proof/dogfood-local-vmci-roi-current-head.log`
- Partial receipt: `/home/brittonr/data/aspen-dogfood-vmci-receipts/dogfood-20260514T201732Z.json`
- Node log: `/home/brittonr/data/aspen-dogfood-vmci/node1.log`

## Verification

- `openspec validate harden-dogfood-vmci-worker-readiness --strict`
- `openspec validate --all --strict --json`
- Future implementation: a focused readiness/fail-fast test plus a successful VM-CI proof on a TAP/TUN-capable host.
