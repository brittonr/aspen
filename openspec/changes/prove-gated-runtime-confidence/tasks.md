## Phase 1: Baseline and cheap product paths

- [x] [serial] Capture clean git status, recent commits, host KVM/TUN/CPU/user prerequisites, and relevant impure check names as baseline evidence for `r[test-harness-runtime.gated-runtime-confidence-sweep.clean-baseline]`. Evidence: `evidence/baseline.md`. ✅ 3m (started: 2026-05-12T22:49:02Z → completed: 2026-05-12T22:52:00Z)
- [x] [depends:baseline] Run cheap runtime-host product-path checks for WASM, OCI lowering, Hyperlight guardrail, and Hermit/uHyve guardrail, preserving command lines, exit statuses, and proof-boundary classification for `r[test-harness-runtime.gated-runtime-confidence-sweep.staged-order]`. Evidence: `evidence/cheap-runtime-host.md`. ✅ 4m (started: 2026-05-12T22:50:10Z → completed: 2026-05-12T22:54:00Z)

## Phase 2: VM and microVM product checks

- [ ] [depends:cheap-product-paths] Run nearby VM/microVM checks beyond the already-restored cluster rails, including `microvm-virtiofs-net-test`, `microvm-net-mesh-test`, `microvm-raft-virtiofs-test`, and `vm-snapshot-virtiofs-test` where host prerequisites allow.
- [ ] [depends:vm-microvm-checks] Record whether each VM/microVM check reached build closure, VM boot, service readiness, product assertions, cached success, or a classified blocker.

## Phase 3: Gated runtime-host execution proofs

- [ ] [depends:vm-microvm-classification] Run the VM runtime-host package gate: `nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L`.
- [ ] [depends:vm-package-gate] Run nested-KVM `vm-snapshot-e2e-test` with sandbox disabled and classify the highest reached boundary: cluster ready, Cloud Hypervisor boot, guest readiness marker, snapshot files, worker registration, job completion, or final runtime-host receipt.
- [ ] [depends:vm-runtime-host-e2e] Run the Hermit/uHyve ignored product proof using the built `uhyve` runner and `hermit-uhyve-marker` fixture, or record the exact host/build blocker.
- [ ] [depends:hermit-uhyve-proof] Run the Hyperlight ignored product-path proof, or record the exact host/build blocker.

## Phase 4: Dogfood and full repository confidence

- [ ] [depends:gated-runtime-proofs] Run `nix run .#dogfood-local -- full` and classify whether it reaches dogfood/self-hosting acceptance with a receipt.
- [ ] [depends:dogfood-full] Run `nix flake check -L --max-jobs 1` only after lower tiers pass or are classified as non-product blockers.

## Phase 5: Evidence and follow-up routing

- [ ] [serial] Redact or omit raw tickets, cookies, credentials, private keys, connection strings, and equivalent secret material from any committed evidence.
- [ ] [depends:all-proof-tiers] Write a compact committed evidence summary with command lines, exit statuses, proof markers, failure stages, and boundary classifications for `r[test-harness-runtime.gated-runtime-confidence-sweep.boundary-classification]` and `r[test-harness-runtime.gated-runtime-confidence-sweep.redacted-evidence]`.
- [ ] [depends:evidence-summary] Create or update follow-up OpenSpecs for any real product behavior failures or multi-component repairs found during the sweep; document any narrow direct build/input drift fixes with focused verification.
- [ ] [depends:follow-up-routing] Run `openspec validate prove-gated-runtime-confidence --strict --json`, `openspec validate --all --strict --json`, and `git diff --check` before completion.
