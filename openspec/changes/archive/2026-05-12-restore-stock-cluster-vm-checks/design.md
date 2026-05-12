## Context

The core cluster evidence is already positive:

- Rust multi-node integration: 4 passed / 0 failed, including leader failover.
- `aspen-cluster`: 219 passed / 0 failed / 3 skipped.
- selected `aspen-raft` cluster/membership/leader/election/replication tests: 55 passed / 0 failed.
- fallback minimal 3-QEMU-VM NixOS proof: passed with three voters and cross-node `cluster health` from every node.

The stock VM rails failed earlier than their product assertions:

- `microvm-cluster-test`: build-input/API mismatch around `iroh`/`portmapper` before VM test execution.
- `multi-node-cluster-test`: external WASM forge plugin fixture API drift around `ForgeRepoInfo`/`PluginInfo` before VM test execution.

## Goals / Non-Goals

**Goals:**

- Make both stock cluster VM checks reach their VM scripts and pass focused execution.
- Keep cluster formation proof independent from optional plugin/forge fixture drift where possible.
- Keep plugin/forge fixture coverage explicit when it runs, with its own failure classification.
- Record concrete command evidence and redacted/log-safe proof markers.

**Non-Goals:**

- Replacing the existing fallback minimal 3-VM proof.
- Broad runtime-host, Hyperlight, Hermit/Uhyve, or full dogfood acceptance.
- Treating an optional plugin fixture compile failure as proof that Raft/Iroh clustering is broken.

## Decisions

### 1. Fix stock checks instead of relying only on fallback proof

**Choice:** Repair the named repository checks that operators already expect to run.

**Rationale:** The fallback minimal 3-VM proof is useful triage evidence, but stock checks should remain durable acceptance rails.

**Alternative:** Keep the fallback proof as the only VM evidence. Rejected because it leaves broken advertised check targets in place.

**Implementation:** Patch the Nix/package/fixture seams that prevent the checks from building, then rerun each focused target.

### 2. Separate core clustering from optional plugin/forge assertions

**Choice:** The multi-node VM test must preserve a core-cluster pass boundary before optional WASM forge/plugin coverage.

**Rationale:** A stale forge plugin fixture should not hide whether node startup, learner addition, voter promotion, and cross-node health work.

**Alternative:** Keep all multi-node coverage behind plugin package success. Rejected because it makes the cluster check hostage to unrelated ABI drift.

**Implementation:** Either update the plugin fixture to current APIs or split/gate plugin install/assertions so core cluster formation has an independent proof marker.

### 3. Classify build-closure failures separately from VM product failures

**Choice:** Evidence must say whether failure happened during Nix build closure, VM boot, cluster formation, plugin install, or post-formation assertions.

**Rationale:** This prevents overclaiming both success and failure.

**Alternative:** Report any `nix build` failure as a cluster failure. Rejected because the observed failures happened before the cluster product path ran.

## Risks / Trade-offs

**External fixture drift** → Prefer in-repo fixture sources or pinned/compatible package inputs when possible; otherwise explicitly mark external fixture coverage as optional and independently diagnosed.

**Heavy VM runtime** → Run focused checks serially with `--impure` and preserve log excerpts/markers rather than full secret-bearing logs.

**Secret leakage** → Redact cluster tickets and avoid committing transient VM logs containing tickets.
