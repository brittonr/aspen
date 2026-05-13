## Phase 1: Baseline and cheap product paths

- [x] [serial] Capture clean git status, recent commits, host KVM/TUN/CPU/user prerequisites, and relevant impure check names as baseline evidence for `r[test-harness-runtime.gated-runtime-confidence-sweep.clean-baseline]`. Evidence: `evidence/baseline.md`. ✅ 3m (started: 2026-05-12T22:49:02Z → completed: 2026-05-12T22:52:00Z)
- [x] [depends:baseline] Run cheap runtime-host product-path checks for WASM, OCI lowering, Hyperlight guardrail, and Hermit/uHyve guardrail, preserving command lines, exit statuses, and proof-boundary classification for `r[test-harness-runtime.gated-runtime-confidence-sweep.staged-order]`. Evidence: `evidence/cheap-runtime-host.md`. ✅ 4m (started: 2026-05-12T22:50:10Z → completed: 2026-05-12T22:54:00Z)

## Phase 2: VM and microVM product checks

- [x] [depends:cheap-product-paths] Run nearby VM/microVM checks beyond the already-restored cluster rails, including `microvm-virtiofs-net-test`, `microvm-net-mesh-test`, `microvm-raft-virtiofs-test`, and `vm-snapshot-virtiofs-test` where host prerequisites allow. Evidence: `evidence/vm-microvm-tier.md`. ✅ 183m (started: 2026-05-12T22:52:26Z → completed: 2026-05-13T01:55:00Z)
- [x] [depends:vm-microvm-checks] Record whether each VM/microVM check reached build closure, VM boot, service readiness, product assertions, cached success, or a classified blocker. Evidence: `evidence/vm-microvm-tier.md`. ✅ 3m (completed: 2026-05-13T01:55:00Z)

## Phase 3: Gated runtime-host execution proofs

- [x] [depends:vm-microvm-classification] Run the VM runtime-host package gate: `nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L`. Evidence: `evidence/vm-runtime-host-phase3.md`. ✅ 1m (completed: 2026-05-13T02:55:19Z)
- [x] [depends:vm-package-gate] Run nested-KVM `vm-snapshot-e2e-test` with sandbox disabled and classify the highest reached boundary: cluster ready, Cloud Hypervisor boot, guest readiness marker, snapshot files, worker registration, job completion, or final runtime-host receipt. Evidence: `evidence/vm-runtime-host-phase3.md`. ✅ 46m (completed: 2026-05-13T02:55:19Z)
- [x] [depends:vm-runtime-host-e2e] Run the Hermit/uHyve ignored product proof using the built `uhyve` runner and `hermit-uhyve-marker` fixture, or record the exact host/build blocker. Evidence: `evidence/hermit-uhyve-phase3.md`. ✅ 9m (started: 2026-05-13T03:07:54Z → completed: 2026-05-13T03:09:33Z)
- [x] [depends:hermit-uhyve-proof] Run the Hyperlight ignored product-path proof, or record the exact host/build blocker. Evidence: `evidence/hyperlight-phase3.md`. ✅ 1m (started: 2026-05-13T03:12:13Z → completed: 2026-05-13T03:12:34Z)

## Phase 4: Dogfood and full repository confidence

- [x] [depends:gated-runtime-proofs] Run `nix run .#dogfood-local -- full` and classify whether it reaches dogfood/self-hosting acceptance with a receipt. Evidence: `evidence/dogfood-full.md`. ✅ 43m (started: 2026-05-13T03:13:15Z → completed: 2026-05-13T03:56:25Z; blocked before acceptance: default `/tmp` capacity, then datapool rerun `git push` timeout at local hook/push boundary)
- [x] [depends:dogfood-full] Run `nix flake check -L --max-jobs 1` only after lower tiers pass or are classified as non-product blockers. Evidence: `evidence/full-flake-check.md`. ✅ 98m (started: 2026-05-13T03:57:12Z → completed: 2026-05-13T05:35:26Z)

## Phase 5: Evidence and follow-up routing

- [x] [serial] Redact or omit raw tickets, cookies, credentials, private keys, connection strings, and equivalent secret material from any committed evidence. Evidence: `evidence/redaction-audit.md`. ✅ 1m (started: 2026-05-13T05:37:54Z → completed: 2026-05-13T05:38:45Z)
- [x] [depends:all-proof-tiers] Write a compact committed evidence summary with command lines, exit statuses, proof markers, failure stages, and boundary classifications for `r[test-harness-runtime.gated-runtime-confidence-sweep.boundary-classification]` and `r[test-harness-runtime.gated-runtime-confidence-sweep.redacted-evidence]`. Evidence: `evidence/summary.md`. ✅ 1m (started: 2026-05-13T05:39:07Z → completed: 2026-05-13T05:40:00Z)
- [ ] [depends:evidence-summary] Create or update follow-up OpenSpecs for any real product behavior failures or multi-component repairs found during the sweep; document any narrow direct build/input drift fixes with focused verification.
- [ ] [depends:follow-up-routing] Run `openspec validate prove-gated-runtime-confidence --strict --json`, `openspec validate --all --strict --json`, and `git diff --check` before completion.
