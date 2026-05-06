## Context

The testing harness family can accelerate future extraction work, but reusable defaults must be clearly separated from madsim/network/patchbay/runtime adapters before it becomes a stable public surface. Prior evidence identified `aspen-testing-core` as the reusable root; this change records the public API/readiness decision with fresh fixtures, dependency graph scans, and readiness checker output.

## Goals / Non-Goals

**Goals:**
- Define the reusable testing-core surface and owner expectations.
- Preserve current Aspen compatibility while proving the targeted reusable/evidence boundary.
- Keep madsim, network, patchbay, VM, and concrete cluster bootstrap helpers behind explicit adapter crates.
- Promote the testing harness family to `extraction-ready-in-workspace` with evidence.

**Non-Goals:**
- Do not publish or split crates out of the Aspen monorepo; license/publication policy remains a blocker.
- Do not rewrite madsim/network/patchbay/runtime adapter APIs.
- Do not broaden scope to unrelated crate families or runtime rewrites.

## Decisions

### 1. `aspen-testing-core` is the canonical reusable root

**Choice:** Portable consumers use `aspen-testing-core` for deterministic in-memory cluster/KV implementations, bounded wait helpers, and generic testing state over foundational/KV/trait crates.

**Rationale:** The positive downstream fixture compiles and runs without root Aspen, cluster runtime, RPC handlers, transport/Raft runtime, concrete Iroh runtime, patchbay, madsim/turmoil, or adapter crates.

**Alternative:** Treat `aspen-testing` as the reusable root. Rejected because it is the compatibility facade for existing suites and can aggregate adapter/runtime-oriented helpers.

### 2. Adapter crates stay explicit

**Choice:** `aspen-testing-madsim`, `aspen-testing-network`, and `aspen-testing-patchbay` remain explicit adapter crates for concrete simulation, namespace, transport, patchbay, and host integration behavior.

**Rationale:** The negative fixture proves adapter crates are not reachable from a consumer that depends only on `aspen-testing-core`, while package checks prove those adapters still compile.

### 3. Lightweight Tokio support is reusable utility support, not a runtime adapter leak

**Choice:** `tokio` `sync`/`time` support remains allowed for `aspen-testing-core` async trait implementations and wait helpers; concrete runtime/transport/cluster dependencies remain forbidden from reusable defaults.

**Rationale:** `aspen-testing-core` implements async traits and bounded wait utilities. The dependency graph excludes concrete runtime app, network, patchbay, madsim, Raft, Iroh, and handler crates.

## Verification Strategy

- For `testing-harness-extraction.testing-core-default-reusable` and `testing-harness-extraction.testing-core-default-reusable.evidence`, run the positive downstream fixture, capture metadata, and record docs/policy/inventory updates.
- For `testing-harness-extraction.adapters-explicit-negative-checked` and `testing-harness-extraction.adapters-explicit-negative-checked.evidence`, run the negative adapter fixture as an expected failure, run the portable dependency graph scan, and run adapter package checks.
- For `testing-harness-extraction.workspace-readiness-evidenced` and `testing-harness-extraction.workspace-readiness-evidenced.evidence`, run the crate-extraction readiness checker for `testing-harness` after docs/policy/inventory are updated.
- Run strict OpenSpec validation, repo preflight, and whitespace checks before committing/archive.

## Risks / Trade-offs

**False adapter confidence** → The dependency-tree scan explicitly checks for root Aspen, cluster runtime, RPC handlers, transport/Raft runtime, concrete Iroh runtime, patchbay, madsim/turmoil, and adapter crates.

**Tokio overclassification** → Tokio `sync`/`time` is documented as reusable utility support for async wait helpers; concrete runtime adapters remain excluded.

**False publication signal** → Readiness is limited to `extraction-ready-in-workspace`; publication/repo-split remains blocked on license policy.
