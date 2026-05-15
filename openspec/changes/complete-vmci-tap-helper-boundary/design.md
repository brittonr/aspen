## Context

`NetworkMode::TapWithHelper` exists in configuration and environment parsing, but validation rejects it and runtime falls back to direct `ip` operations. The current setup script pre-creates bridge/TAP devices, yet `aspen-node` still tries to attach/up/delete TAP links without `CAP_NET_ADMIN`, causing `RTNETLINK answers: Operation not permitted` during live VM-CI dogfood startup.

## Goals / Non-Goals

**Goals:**

- Keep `aspen-node` unprivileged for VM-CI dogfood.
- Provide a scoped helper command for TAP attach/up/delete and optional create.
- Restrict helper inputs to Aspen CI device names and the expected bridge.
- Preserve direct `tap` mode for hosts that intentionally run with `CAP_NET_ADMIN`.
- Preserve `none`/`isolated` mode for offline VM proofs.

**Non-Goals:**

- General-purpose network administration helper.
- Arbitrary bridge/device management.
- Replacing Cloud Hypervisor networking or VM ticket/bootstrap behavior.

## Decisions

### Helper CLI

**Choice:** Add `aspen-tap-helper` as a small Rust binary in `aspen-ci-executor-vm`. It accepts subcommands for `ensure` and `delete` and delegates to `ip` after validating names.

**Rationale:** Rust gives testable allowlist parsing and avoids embedding privileged policy in shell. The helper can be copied out of the Nix store by `setup-ci-network` and granted `cap_net_admin+ep` on the mutable copy.

**Alternative:** Run `aspen-node` with `CAP_NET_ADMIN`; rejected because it grants the full node broad network mutation authority.

### Runtime integration

**Choice:** `TapWithHelper` calls the helper for TAP ensure/delete, then passes `tap=<name>` to Cloud Hypervisor like direct TAP mode.

**Rationale:** Cloud Hypervisor does not need broad privileges when the TAP is prepared by the helper. This is the smallest implementation that matches the existing mode and failure evidence.

### Dogfood defaults

**Choice:** `dogfood-local-vmci` sets `ASPEN_CI_NETWORK_MODE=tap-helper` and `ASPEN_CI_TAP_HELPER_PATH=/tmp/aspen-ci-tap-helper` when that helper exists and is executable, unless the operator explicitly set network mode/helper env vars.

**Rationale:** The setup app remains the privileged boundary, while the dogfood app remains unprivileged and deterministic.

## Risks / Trade-offs

- **File capabilities on copied helper** → `setup-ci-network` installs the helper to `/tmp/aspen-ci-tap-helper` and best-effort applies `setcap`; readiness still fails fast if unavailable.
- **Stale TAP cleanup** → helper delete is allowlisted and idempotent.
- **Helper path spoofing** → readiness requires an existing helper path; the helper itself enforces command/device/bridge policy.

## Validation Plan

- Unit tests for helper allowlists and config validation.
- Unit tests that `TapWithHelper` command paths reject missing helpers and choose helper operations.
- `cargo test -p aspen-ci-executor-vm tap_helper -- --nocapture`.
- `cargo test -p aspen-dogfood vmci_readiness -- --nocapture`.
- `openspec validate complete-vmci-tap-helper-boundary --strict --json` and all-spec validation.
- Live `nix run .#dogfood-local-vmci -- --cluster-dir /home/brittonr/data/aspen-dogfood-vmci full` if helper installation succeeds on this host.
