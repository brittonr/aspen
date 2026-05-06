# Verification: dogfood-current-head-acceptance-receipt

## Implementation Evidence

- Changed file: `docs/operator-receipts.md`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/tasks.md`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-20260506T191239Z.receipt.redacted.json`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-full-current-head-2f55.redacted.log`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-show-current-head-2f55.txt`
- Changed file: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-diagnose-current-head-2f55.txt`

Current-head dogfood was run from clean, synced `main` at commit `2f55a92e17b3abecb71c5fa2f96eca087281fb1a` with:

```bash
ASPEN_NODE_BIN=$PWD/target/debug/aspen-node \
GIT_REMOTE_ASPEN_BIN=$PWD/target/debug/git-remote-aspen \
cargo run -p aspen-dogfood -- --cluster-dir /tmp/aspen-dogfood full --leave-running
```

The run produced durable local receipt `/tmp/aspen-dogfood-receipts/dogfood-20260506T191239Z.json`, copied into this change as `evidence/dogfood-20260506T191239Z.receipt.redacted.json`. The receipt records schema `aspen.dogfood.run-receipt.v1`, run id `dogfood-20260506T191239Z`, git commit `2f55a92e17b3abecb71c5fa2f96eca087281fb1a`, stage timings, and first failure category/message.

The loop reached cluster start and Forge push, then deliberately gated at native CI `clippy` in stage `check`. The local flake reproduction is captured as a failure rail: `nix build .#checks.x86_64-linux.clippy --no-link -L --show-trace` exits 1 with unresolved `netlink-packet-route` imports against the Nix vendored `netlink-packet-core` surface. Because the build stage failed, no deploy/verify/publish_receipt stages ran and no cluster-backed receipt key was published; the local receipt and readback/diagnose outputs are the operator evidence for this gated current-head run.

Redaction notes:

- Cluster ticket output is redacted as `[REDACTED]` in the saved dogfood log.
- `aspen://...` remote URLs are redacted in the saved dogfood log.
- The JSON receipt and show/diagnose outputs contain commit ids, run ids, local paths, stage statuses, and failure category/message only; no bearer tokens, cookies, private keys, or cluster tickets are preserved.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `dogfood-current-head-acceptance-receipt`.
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/proposal.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/design.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/specs/dogfood-evidence/spec.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/tasks.md`
- [x] Confirm clean current head and identify the exact dogfood command and receipt paths.
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-20260506T191239Z.receipt.redacted.json`
- [x] Run or deliberately gate the full dogfood loop; capture success/failure receipt, local readback, cluster readback if applicable, and diagnostics.
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-full-current-head-2f55.redacted.log`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-show-current-head-2f55.txt`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-diagnose-current-head-2f55.txt`
- [x] Update operator docs/evidence with commit-bound receipt, redaction notes, and acceptance/failure triage.
  - Evidence: `docs/operator-receipts.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`
- [x] Sync/archive only after every implementation/evidence task is complete.
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/tasks.md`
  - Evidence: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| receipt schema | `cargo test -p aspen-dogfood receipt -- --nocapture` | pass | terminal transcript | Covers receipt validation/readback changes that add commit-bound evidence. | Full dogfood success after the Nix clippy/vendor blocker is fixed. |
| runtime binaries | `cargo build --features node-runtime-apps,ci,docs,blob,hooks,shell-worker,automerge,secrets,proxy,git-bridge --bin aspen-node --bin git-remote-aspen` | pass | terminal transcript | Builds the exact binaries used by the cargo-based dogfood run. | Nix `dogfood-local` once vendor clippy passes. |
| format | `nix fmt . -- --check` | pass | terminal transcript | Matches the dogfood CI `format-check` job and proves the prior format gate is repaired. | Keep running before dogfood retries. |
| dogfood | `ASPEN_NODE_BIN=$PWD/target/debug/aspen-node GIT_REMOTE_ASPEN_BIN=$PWD/target/debug/git-remote-aspen cargo run -p aspen-dogfood -- --cluster-dir /tmp/aspen-dogfood full --leave-running` | expected fail at CI clippy | `evidence/dogfood-full-current-head-2f55.redacted.log`, `evidence/dogfood-20260506T191239Z.receipt.redacted.json` | Proves current head starts the dogfood cluster, pushes current git HEAD to Forge, and emits a commit-bound receipt when the full loop gates. | Fix Nix vendored netlink clippy, rerun full dogfood to deploy/verify/publish_receipt success. |
| receipt readback | `cargo run -q -p aspen-dogfood -- receipts show/diagnose /tmp/aspen-dogfood-receipts/dogfood-20260506T191239Z.json` | pass | `evidence/receipt-show-current-head-2f55.txt`, `evidence/receipt-diagnose-current-head-2f55.txt` | Proves operators can inspect the local receipt without scraping raw logs. Cluster readback is not applicable because publish_receipt did not run after build failure. | `receipts cluster-show <run-id> --json` on a successful leave-running run. |
| clippy blocker | `nix build .#checks.x86_64-linux.clippy --no-link -L --show-trace` | expected fail | `/tmp/aspen-nix-clippy-current.log` | Reproduces the dogfood CI `clippy` blocker locally and identifies the failing Nix vendor/import seam. | Continue Nix vendor fix in a follow-up or next drain slice. |
| whitespace | `git diff --check` | pass | terminal transcript | Ensures committed evidence/docs/spec files are whitespace-clean. | Run staged diff check before final commit/archive. |

## Verification Commands

### `cargo test -p aspen-dogfood receipt -- --nocapture`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

### `cargo build --features node-runtime-apps,ci,docs,blob,hooks,shell-worker,automerge,secrets,proxy,git-bridge --bin aspen-node --bin git-remote-aspen`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

### `nix fmt . -- --check`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

### `ASPEN_NODE_BIN=$PWD/target/debug/aspen-node GIT_REMOTE_ASPEN_BIN=$PWD/target/debug/git-remote-aspen cargo run -p aspen-dogfood -- --cluster-dir /tmp/aspen-dogfood full --leave-running`
- Status: expected failure at native CI clippy gate
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-full-current-head-2f55.redacted.log`
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-20260506T191239Z.receipt.redacted.json`

### `cargo run -q -p aspen-dogfood -- receipts show /tmp/aspen-dogfood-receipts/dogfood-20260506T191239Z.json`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-show-current-head-2f55.txt`

### `cargo run -q -p aspen-dogfood -- receipts diagnose /tmp/aspen-dogfood-receipts/dogfood-20260506T191239Z.json`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/receipt-diagnose-current-head-2f55.txt`

### `nix build .#checks.x86_64-linux.clippy --no-link -L --show-trace`
- Status: expected failure: Nix vendored netlink-packet-route/core import seam
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/dogfood-full-current-head-2f55.redacted.log`

### `openspec validate dogfood-current-head-acceptance-receipt --strict`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

### `scripts/openspec-preflight.sh dogfood-current-head-acceptance-receipt`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`

### `git diff --check`
- Status: pass
- Artifact: `openspec/changes/dogfood-current-head-acceptance-receipt/verification.md`
