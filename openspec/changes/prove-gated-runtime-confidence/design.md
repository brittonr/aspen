## Context

The current checkout has already passed:

- restored stock cluster VM rails (`microvm-cluster-test`, `multi-node-cluster-test`), with a clean-HEAD rerun;
- core cluster/Raft checks and `cargo nextest run -P quick`;
- `scripts/test-harness.sh quick-confidence --summary target/quick-confidence/summary.json`.

The quick-confidence summary explicitly skipped full dogfood/self-hosting acceptance, KVM/NixOS runtime-host proofs, Uhyve/Hermit runtime-host execution proofs, Hyperlight runtime-host execution proofs, network/ignored nextest profiles, and full `nix flake check`.

A fresh host probe shows `/dev/kvm`, `/dev/net/tun`, a virtualization CPU flag (`svm`), and user membership in `kvm`, so gated VM proofs are worth attempting on this machine.

## Goals / Non-Goals

**Goals:**

- Execute a staged proof sweep from cheap runtime-host product paths through gated VM/runtime-host/dogfood checks.
- Preserve compact evidence and classify each boundary without overclaiming.
- Split true product failures into follow-up OpenSpecs when scope exceeds a narrow local fix.

**Non-Goals:**

- Make quick-confidence claim full production readiness.
- Force all expensive checks into one monolithic command.
- Commit full raw logs containing tickets or noisy diagnostic dumps.
- Repair broad product failures inside this proof-sweep change without a follow-up spec.

## Decisions

### 1. Use staged tiers instead of a single mega-gate

**Choice:** Run proof tiers in this order: baseline/host probe, cheap product-path checks, nearby VM/microVM checks, runtime-host E2E, Hermit/uHyve and Hyperlight execution, dogfood/self-hosting, then full flake.

**Rationale:** Earlier tiers are cheaper, isolate build/input drift from product failures, and avoid wasting nested-KVM/dogfood time when basic product-path checks are already broken.

**Alternative:** Run `nix flake check -L` or dogfood first. Rejected because failures would be slower and harder to classify.

### 2. Treat proof boundaries as first-class evidence

**Choice:** Every task records the command, exit status, proof marker or failure stage, and boundary classification.

**Rationale:** Aspen needs operator-trust evidence, not just pass/fail claims. Cached Nix success, static readiness, and true runtime receipts must remain distinguishable.

**Alternative:** Keep evidence only in chat. Rejected because it is not durable or reviewable.

### 3. Keep raw secrets out of committed evidence

**Choice:** Store raw transient logs only under ignored `target/runtime-proof/`; commit compact redacted summaries if needed.

**Rationale:** VM and cluster tests may print tickets or connection material.

**Alternative:** Commit full logs. Rejected due credential/ticket exposure and noise.

### 4. Split repair work from proof classification

**Choice:** If a real product failure appears, create/update a targeted OpenSpec before implementing the fix unless it is a narrow build/input drift repair.

**Rationale:** The proof campaign should not become an unbounded implementation sink.

**Alternative:** Fix everything inline. Rejected because broad runtime-host, dogfood, or VM repair could span multiple components and sessions.

## Validation Plan

- Validate this change with OpenSpec strict validation and whitespace checks.
- During execution, write or update task evidence with exact command lines, exit statuses, proof markers, and boundary classifications.
- Use `target/runtime-proof/` for raw local logs and redacted summaries for committed evidence.
- Before marking the change complete, run `openspec validate prove-gated-runtime-confidence --strict --json`, `openspec validate --all --strict --json`, and `git diff --check`.

## Risks / Trade-offs

**Long-running checks** → Use managed background execution and stop at actionable blockers.

**Host-specific failures** → Capture KVM/TUN/CPU/user state and classify host blockers separately.

**Cached Nix outputs** → Report cached success as command success unless explicit proof markers/logs are available; force rebuild only when a fresh runtime proof is required.

**Secret leakage** → Redact tickets and equivalent secret material before committing evidence.
