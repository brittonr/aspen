## Context

The latest VMCI dogfood retry no longer fails at host health, guest reachability, route retention, worker registration, job dequeue, or executor start. It reaches `vm_ci_boundary=job_result_published`, but the `clippy` job fails because its workspace path does not contain `flake.nix`.

The local/adapter CI path in `crates/aspen-ci/src/adapters.rs` creates a source archive through `create_source_archive(checkout_dir, blob_store)` and stores the hash in `PipelineContext.source_hash`. The RPC trigger path in `crates/aspen-ci-handler/src/handler/pipeline.rs` checks out and prepares the source tree, then builds a trigger context with `source_hash: None`.

VMs cannot read host checkout paths. They require the source archive hash to seed their per-job workspace from the blob store.

## Goals / Non-Goals

**Goals:**
- Make `CiTriggerPipeline` produce the same VM-materializable `source_hash` as the adapter-triggered path.
- Preserve source archive hashes through nix-job payload transformation and local executor workspace seeding.
- Fail/classify missing source archive or missing `flake.nix` at the source-materialization boundary.
- Prove the fix with focused tests and a live VMCI dogfood retry.

**Non-Goals:**
- Redesigning CI pipeline orchestration.
- Changing external CI RPC schemas unless required by existing internal context plumbing.
- Fixing unrelated build failures after the workspace root contains source.

## Decisions

### 1. Archive source in the RPC handler before enqueueing jobs

**Choice:** After `checkout_trigger_repository` succeeds and before `orchestrator.execute(...)`, the RPC handler should create a source archive using the orchestrator blob store and pass `Some(hash)` into `build_trigger_context`.

**Rationale:** This mirrors the working adapter path and keeps VM workspace materialization driven by the existing `source_hash` contract.

**Alternative rejected:** Teach VM workers to clone/fetch directly from Forge. That would add network and authentication surface inside isolated workers and duplicate existing archive plumbing.

**Implementation:** Add a helper near `build_trigger_context` or reuse/expose an existing source archive helper. Keep failure policy explicit: if VM-capable CI requires archive materialization and archive creation fails, return a targeted trigger/build error or classify the run as source materialization failure rather than allowing a later missing-flake error.

### 2. Validate materialized workspace root before running nix

**Choice:** For jobs carrying `source_hash` and `flake_url = "."`, validate that materialization produced a root `flake.nix` before invoking `nix build`.

**Rationale:** The current failure only appears as a nix error after command launch. A preflight gives deterministic boundary evidence and faster triage.

**Alternative rejected:** Rely only on nix output. That conflates archive propagation, archive unpacking, cwd rewriting, and actual flake evaluation.

### 3. Keep diagnostics bounded and redacted

**Choice:** Evidence should record booleans and bounded root-entry summaries, not full directory dumps or secrets.

**Rationale:** Dogfood receipts and VMCI diagnostics are operator-facing and may be persisted.

## Risks / Trade-offs

**Archive creation cost** → Source archiving may add time to RPC-triggered runs. Mitigate by using existing archive code and bounded source preparation already used in adapter path.

**Feature-gated blob availability** → Some CI modes may not have a blob store. Mitigate with explicit behavior: either skip source archive only for non-VM-compatible modes or fail fast when VM jobs would require it.

**Nested archive root** → Archive unpacking may place source under a subdirectory. Mitigate with a test that materializes by hash and asserts root `flake.nix` exists at the executor working directory.

## Validation Plan

- Unit/integration test that RPC trigger context includes `source_hash` when a blob store is configured.
- Materialization test that archive hash unpacks to a root containing `flake.nix`.
- Existing VM nix payload transform test continues to prove `source_hash` is preserved.
- `nix run .#rustfmt`.
- Focused `cargo test` targets for CI handler/source archive and VM worker transform.
- `openspec validate propagate-rpc-ci-source-archive --strict --json`.
- Live `nix run .#dogfood-local-vmci -- full` retry; success or next-boundary failure must no longer be missing root `flake.nix` due absent `source_hash`.
