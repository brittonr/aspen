## Context

Aspen already has pure cores for cluster planning, cluster lifecycle receipts, lifecycle drift summaries, multinode scenario metadata, local multiprocess run receipts, VM shard receipts, and failure repro bundles. The gap is execution wiring: the `cluster` CLI currently exposes useful per-node receipt files and rendered output, while the cluster-level review receipt and fixture-derived run comparison are not the primary operator artifact.

## Decisions

### 1. Receipt-first cluster workflow command

**Choice:** Add a cluster harness workflow that runs or observes `init`, `start`, `status`, and `stop` phases from an explicit fixture-derived plan and writes a canonical `cluster-lifecycle-run-v1` receipt to a requested output path or run directory.

**Rationale:** Existing per-node receipts remain useful child evidence, but reviewers need one stable parent artifact that binds manifest ref, node order, phase decisions, already-running observations, stop order, diagnostics, and caveats. Rendered stdout remains a view.

### 2. Durable run artifact directory

**Choice:** A cluster run directory contains the fixture metadata, derived plan, lifecycle receipt, per-node child receipts or refs, reconciliation receipt, drift summary, diagnostic-log refs, and failure bundle when denied. The directory is verified offline by recomputing refs and comparing artifact kinds against fixture metadata.

**Rationale:** A directory contract makes CI outputs, local reproduction, and review portable without granting ambient filesystem state any authority.

### 3. Executable local multiprocess shell over pure plan core

**Choice:** Keep planning and validation pure in `src/testing/multinode`, and add a thin shell that spawns child `molten node` or `molten cluster` processes only after the plan passes. The shell records startup, workflow, shutdown, timeout, orphan, and cleanup observations and finalizes the existing local multiprocess executable-run receipt.

**Rationale:** The local multiprocess tier should catch process isolation and cleanup bugs before VM checks while preserving functional-core / imperative-shell boundaries.

### 4. Fixture-driven execution and offline verification share one comparator

**Choice:** The same pure comparison core evaluates fixture metadata against observed run artifacts for cluster lifecycle, local multiprocess, and VM shards. It denies on topology drift, command-surface drift, artifact-kind drift, missing required receipts, unsupported pass claims, undeclared variance, or caveat mismatch.

**Rationale:** One comparator prevents the fixture from becoming documentation only. It also avoids separate handwritten logic for CLI, local multiprocess, and VM runners.

### 5. First-divergence failure triage stays diagnostic-only

**Choice:** Denied cluster runs emit a first-divergence summary naming the first missing or mismatched semantic field and may export a sealed diagnostic failure bundle. The summary and bundle cannot satisfy pass gates.

**Rationale:** Operators need actionable failures, but diagnostics and logs must not become authority, replay, VM, or production-readiness evidence.

## Functional core / shell split

- Pure core: derive fixture-backed run plans, validate artifact-directory manifests, compare observed artifacts against fixture metadata, produce lifecycle/drift/first-divergence diagnostics, and build receipts from in-memory observations.
- Shell: read Nickel exports and Preserves files, spawn child processes, manage temp/state roots, enforce process timeouts, collect logs, write artifact directories, and print summaries.

## Risks / Trade-offs

- The run directory contract adds more artifacts to maintain, but it replaces ambiguous stdout and scattered receipt discovery.
- Local multiprocess execution is not VM or WAN evidence; receipts must preserve that caveat in every pass path.
- Offline verification must reject unknown artifact kinds by default so new receipt shapes require an explicit reviewed change.
