# Verification: VM-CI TAP helper boundary

## Focused checks

```bash
bash -n scripts/setup-ci-network.sh scripts/dogfood-local-vmci.sh
cargo test -p aspen-dogfood vmci_readiness -- --nocapture
cargo test -p aspen-ci-executor-vm --bin aspen-tap-helper -- --nocapture
nix eval --raw .#apps.x86_64-linux.dogfood-local-vmci.program
nix eval --raw .#apps.x86_64-linux.setup-ci-network.program
openspec validate complete-vmci-tap-helper-boundary --strict --json
git diff --check
```

Results on 2026-05-15:

- shell syntax checks passed.
- `aspen-dogfood` VM-CI readiness tests passed: 8 passed.
- `aspen-tap-helper` allowlist tests passed: 6 passed.
- Nix app evals for `dogfood-local-vmci` and `setup-ci-network` succeeded.
- OpenSpec strict validation for this change passed.
- whitespace check passed.

## Host boundary / live acceptance

The post-implementation host probe selected the new default helper path:

```text
helper=/usr/local/libexec/aspen-ci-tap-helper
helper_executable=no
/dev/kvm present: yes
/dev/net/tun present: yes
bridge_aspen_ci_br0=yes
CapEff: 0000000000000000
```

Attempting the privileged setup boundary from the agent session failed without mutating host networking:

```bash
sudo -n nix run .#setup-ci-network
# sudo: a password is required
```

Highest verified boundary: implementation, allowlist/readiness tests, Nix app evaluation, and host prerequisite probe are verified. Live VM-CI dogfood acceptance remains operator-gated until `sudo nix run .#setup-ci-network` installs `/usr/local/libexec/aspen-ci-tap-helper` with effective `cap_net_admin+ep`, followed by `nix run .#dogfood-local-vmci -- --cluster-dir /home/brittonr/data/aspen-dogfood-vmci full`.
