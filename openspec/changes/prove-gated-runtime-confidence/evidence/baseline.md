# Baseline evidence

Captured: 2026-05-12T22:49:02Z

## Command

```bash
# summarized from target/runtime-proof/gated-runtime-baseline.log
git status --short --branch
git log --oneline -5
ls -l /dev/kvm /dev/net/tun 2>&1 || true
grep -m1 -E 'vmx|svm' /proc/cpuinfo || true
id
groups
nix eval --impure --json .#checks.x86_64-linux --apply builtins.attrNames
```

## Git baseline

The checkout was clean at `## main...origin/main` before starting the OpenSpec drain. The captured log shows only the in-progress task update after marking the baseline task active.

Recent commits at capture time:

```text
e973c63e8 openspec: plan gated runtime confidence proof
a7a3d7ba7 openspec: archive stock cluster VM checks
39b330b05 fix: restore stock cluster VM checks
e0a967f2b openspec: track stock cluster VM check repairs
5105fcc4b chore: gate raft network adapter boundary
```

## Host prerequisite classification

- `/dev/kvm`: present, mode `crw-rw-rw-`, group `kvm`.
- `/dev/net/tun`: present, mode `crw-rw-rw-`.
- CPU virtualization flag: present (`svm`).
- User: `brittonr`, member of `kvm`.

Classification: host prerequisites are sufficient to attempt KVM/TUN-gated VM proofs on this machine. Future VM failures should not be classified as host-capability blockers unless a check reports a more specific permission or runtime environment failure.

## Relevant impure proof/check attributes

```text
ci-dogfood-deploy-multinode-test
ci-dogfood-deploy-test
ci-dogfood-full-loop-test
ci-dogfood-full-workspace-test
ci-dogfood-self-build-test
ci-dogfood-test
ci-dogfood-workspace-test
cluster-docs-peer-test
dogfood-binary-smoke-test
federation-ci-dogfood-test
forge-cluster-test
hermit-uhyve-marker-contract
microvm-aspen-node-test
microvm-cluster-test
microvm-net-mesh-test
microvm-nginx-test
microvm-raft-virtiofs-test
microvm-virtiofs-net-test
microvm-virtiofs-stress-test
multi-node-cluster-test
multi-node-dogfood-test
multihost-microvm-mesh-test
test-aspen-cluster
test-aspen-cluster-bridges
test-aspen-cluster-handler
test-aspen-cluster-types
vm-snapshot-e2e-test
vm-snapshot-virtiofs-test
```

## Boundary

This evidence only proves baseline state and host capability prerequisites. It does not prove any runtime-host execution, dogfood acceptance, VM product behavior, or full-flake confidence by itself.
