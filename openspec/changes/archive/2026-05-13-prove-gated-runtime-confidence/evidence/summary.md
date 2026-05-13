# Gated runtime confidence summary

Captured: 2026-05-13T05:39:07Z

This compact summary satisfies `r[test-harness-runtime.gated-runtime-confidence-sweep.boundary-classification]` and `r[test-harness-runtime.gated-runtime-confidence-sweep.redacted-evidence]`. Detailed command summaries are in the per-tier evidence files; raw logs remain ignored under `target/runtime-proof/`.

## Baseline

Evidence: `evidence/baseline.md`.

- Clean baseline and host prerequisites were captured before product proofs.
- Host classification: sufficient KVM/TUN/user prerequisites to attempt KVM/TUN-gated VM proofs on this machine.
- Relevant impure proof/check attributes were enumerated, including runtime-host and marker-contract rails.

## Phase 1: cheap product paths

Evidence: `evidence/cheap-runtime-host.md`.

- WASM product path: exit 0, product-path marker passed through JobManager/WorkerPool/WASM worker orchestration, including invalid-WASM negative path.
- OCI lowering product path: exit 0, product marker passed for OCI-lowered WASM execution; model-only/raw-container paths did not satisfy proof markers.
- Hyperlight non-ignored guardrail: exit 0, marker distinction passed; classified as guardrail/static distinction, not execution proof.
- Hermit/uHyve non-ignored guardrails: exit 0, fake-runner and negative-path receipt behavior passed; classified as guardrail/static wrapping, not real Uhyve execution.

## Phase 2: VM and microVM product checks

Evidence: `evidence/vm-microvm-tier.md`.

Focused build-input drift fixes were made and verified with `git diff --check` plus Nix eval before rerunning product checks.

- `microvm-virtiofs-net-test`: exit 0; reached 3-node Raft cluster, VirtioFS data path, mesh publication, guest nginx readiness, SOCKS5/iroh/curl product assertion.
- `microvm-net-mesh-test`: exit 0; reached 3-node Raft cluster, SOCKS5 proxy, guest server/client microVMs, mesh publication, and routed traffic assertion.
- `microvm-raft-virtiofs-test`: exit 0; reached Raft cluster, VirtioFS daemon, guest nginx, and end-to-end Raft → VirtioFS → nginx → curl assertion.
- `vm-snapshot-virtiofs-test`: exit 0 with sandbox disabled; reached Cloud Hypervisor boot, VirtioFS readiness, snapshot creation, restore path, cleanup, and final test-driver success.

Boundary: Phase 2 product assertions passed; these checks did not by themselves claim runtime-host execution receipts.

## Phase 3: gated runtime-host execution proofs

Evidence: `evidence/vm-runtime-host-phase3.md`, `evidence/hermit-uhyve-phase3.md`, and `evidence/hyperlight-phase3.md`.

- `nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L`: exit 0; VM runtime-host package closure builds.
- `nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false`: exit 0 after local executor/package-input correction; reached final VM runtime-host E2E receipt boundary for the snapshot worker path.
- Hermit/uHyve proof: marker package and `hermit-uhyve-marker-contract` built/passed, then ignored test `hermit_uhyve_executes_declared_fixture_through_product_orchestration` exited 0. Boundary: reached final Hermit/uHyve runtime-host product receipt, with marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` observed through packaged Uhyve execution.
- Hyperlight proof: ignored test `hyperlight_job_executes_declared_fixture_through_product_orchestration` exited 0. Boundary: reached final Hyperlight runtime-host product receipt, with marker `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED` asserted through Aspen JobManager/WorkerPool orchestration.

## Phase 4: dogfood and full repository confidence

Evidence: `evidence/dogfood-full.md` and `evidence/full-flake-check.md`.

- Requested `nix run .#dogfood-local -- full`: exit 1; receipt `/tmp/aspen-dogfood-receipts/dogfood-20260513T034410Z.json`; failed at `push` after host `/tmp` capacity pressure. Classification: host/environment capacity blocker before dogfood acceptance, not product proof failure.
- Rerun with larger data path: `nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-proof full`: exit 1; receipt `/home/brittonr/data/aspen-dogfood-proof-receipts/dogfood-20260513T034555Z.json`; reached cluster start, Forge repo creation, CI watch registration, and beginning of `git push`, then timed out at the local hook/push boundary before build/deploy/verify. Classification: real follow-up needed for local dogfood push/hook acceptance; not a completed self-hosting acceptance proof.
- `nix flake check -L --max-jobs 1`: exit 0; terminal marker `all checks passed!`. Boundary: configured local `x86_64-linux` flake checks passed; Nix omitted incompatible systems (`aarch64-darwin`, `aarch64-linux`, `x86_64-darwin`).

## Redaction boundary

Evidence: `evidence/redaction-audit.md`.

- Committed evidence contains command summaries, receipt paths, exit statuses, proof markers, and classifications.
- Raw tickets, cookies, credentials, private keys, connection strings, verbose VM traces, and trust-share byte arrays are omitted from committed evidence.
- Pattern scan of committed evidence found only redaction-documentation matches and no private-key markers, long hex blobs, URI userinfo credentials, or provider key prefixes.

## Overall classification

Aspen's staged runtime confidence proof is strong for local `x86_64-linux` product paths: cheap runtime paths, VM/microVM product checks, VM snapshot runtime-host E2E, Hermit/uHyve, Hyperlight, and full local flake checks passed. Dogfood/self-hosting remains classified rather than accepted: the run progressed beyond host storage pressure with a larger data path but still stopped at the local Forge/CI `git push` hook boundary before self-hosted build/deploy/verify acceptance.
