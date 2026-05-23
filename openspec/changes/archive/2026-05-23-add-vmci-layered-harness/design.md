## Context

Recent VM-CI debugging isolated multiple boundaries: host socket health, direct-only route retention, RPC source archive/source hash propagation, workspace blob streaming/materialization, executor fail-fast behavior, and command progress visibility. The new shell, Nix, and Cargo smoke rails proved the lower layers cheaply, but they are not yet modeled as a durable harness contract.

The current full VMCI run can still sit after Forge repo creation without a structured indication of whether it is pushing source, creating the source archive, triggering CI, waiting for VM workers, or blocked elsewhere. That opacity makes full dogfood too expensive as a primary debugging loop.

## Goals / Non-Goals

**Goals:**

- Provide named VMCI rails that test one boundary layer at a time.
- Emit JSON receipts with explicit phase start/finish/status and last observed boundary.
- Classify source push/archive/trigger stalls distinctly from VM registration, workspace materialization, executor, and CI command failures.
- Keep receipts bounded and redacted.
- Preserve the existing full VMCI rail as final acceptance.

**Non-Goals:**

- Optimizing full clippy runtime.
- Changing Aspen CI scheduling semantics.
- Introducing non-Iroh networking.
- Persisting unredacted tickets, credentials, raw environment, or raw command arguments.

## Decisions

### 1. Use the dogfood binary as the harness shell

**Choice:** Add harness semantics to `aspen-dogfood` rather than introducing a separate tool.

**Rationale:** Dogfood already owns cluster startup, Forge push, CI wait, receipts, and diagnostics. Keeping the harness there avoids duplicating VMCI setup and lets existing Nix apps wrap the same binary.

**Alternative:** A separate `aspen-vmci-harness` binary. Rejected for now because it would duplicate dogfood state handling and likely drift from the real acceptance path.

**Implementation:** Add a `vm-ci-harness` command or equivalent normalized rail path that maps rails to injected CI profiles and common phase receipt recording.

### 2. Treat each boundary as a receipt phase

**Choice:** Model boundaries as structured phases with timestamps, status, artifacts, and failure class.

**Rationale:** Operators should not need to grep node or serial logs to answer which boundary failed. Logs remain the forensic detail, but the receipt is the first-line diagnostic.

**Implementation:** Extend dogfood receipts or add a companion VMCI receipt containing phases such as `cluster_start`, `health_check`, `forge_repo_create`, `source_push`, `source_archive`, `ci_trigger`, `vm_registration`, `job_assignment`, `workspace_materialization`, `executor_command`, and `job_result`.

### 3. Keep layered rails cheap and deterministic

**Choice:** Rails should avoid accidentally evaluating the full Aspen workspace unless the rail is explicitly full.

**Rationale:** The first Cargo smoke attempt failed because `cargo check -p aspen-dogfood` loaded workspace dependencies with host-specific relative paths. A harness rail should test guest Cargo availability and source materialization without conflating those with full workspace topology.

**Implementation:** Shell/Nix/Cargo rails use minimal commands that prove the intended boundary while still checking the materialized source root exists. Full workspace checks stay in the full CI rail.

### 4. Redact first, classify second

**Choice:** Receipts and summaries must redact tickets, secrets, raw env values, and credential-like strings before writing durable evidence.

**Rationale:** VMCI diagnostics are durable artifacts and may be committed or shared as proof. They should remain safe by construction.

**Implementation:** Reuse existing redaction helpers and add tests asserting no ticket-like or env/raw-args content appears in progress markers or receipts.

## Risks / Trade-offs

**Receipt schema churn** → Mitigate by versioning the VMCI receipt schema and preserving existing run receipt fields.

**False confidence from smoke rails** → Mitigate by naming rails by boundary and keeping full VMCI as the only final acceptance rail.

**More code in dogfood** → Mitigate with functional-core/imperative-shell split: pure rail/profile/phase classification logic with thin CLI orchestration.

**Timeout flakiness** → Mitigate by making timeouts configurable while keeping defaults short enough to fail fast in local debugging.
