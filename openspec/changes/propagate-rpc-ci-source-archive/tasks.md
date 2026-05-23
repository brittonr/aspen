## Phase 1: Locate and preserve source archive contract

- [x] [serial] Inventory RPC trigger, adapter trigger, orchestrator context, and VM worker payload paths for `source_hash` ownership and feature gates.
- [x] [serial] Decide fail-fast policy for RPC-triggered VM-capable pipelines when the orchestrator lacks a blob store or source archive creation fails.

## Phase 2: Implement RPC source archive propagation

- [x] [depends:Phase 1] Add or reuse a helper that creates a source archive from the RPC checkout directory and returns a stable source hash.
- [x] [depends:Phase 1] Pass the source hash into `build_trigger_context` before `orchestrator.execute(...)` so jobs are enqueued with VM-materializable source.
- [x] [depends:Phase 1] Preserve existing non-VM/non-blob behavior explicitly, with targeted errors or diagnostics instead of silent `source_hash: None` for VM jobs.

## Phase 3: Workspace materialization diagnostics

- [ ] [depends:Phase 2] Add bounded preflight evidence around VM/local executor workspace seeding: source hash present, materialization attempted, root `flake.nix` present, and bounded root entries on failure.
- [x] [depends:Phase 2] Classify missing archive or missing root `flake.nix` as source/workspace materialization failure in VMCI diagnostics and dogfood receipts.

## Phase 4: Regression tests

- [ ] [depends:Phase 2] Add a focused regression that `handle_trigger_pipeline` creates a run context with non-empty `source_hash` when a blob store is configured.
- [x] [depends:Phase 2] Add a materialization regression proving the source archive referenced by the RPC trigger hash unpacks to a root containing `flake.nix`.
- [ ] [depends:Phase 3] Extend VM nix payload/workspace tests to prove `source_hash` is preserved through transformation and used before `nix build` runs.

## Phase 5: Verification and live retry

- [x] [depends:Phase 4] Run `nix run .#rustfmt`.
- [x] [depends:Phase 4] Run focused Cargo tests for CI handler/source archive propagation and VM worker workspace materialization.
- [x] [depends:Phase 4] Run `openspec validate propagate-rpc-ci-source-archive --strict --json`.
- [ ] [depends:Phase 5] Re-run `nix run .#dogfood-local-vmci -- full` and record whether VMCI passes or advances beyond missing `flake.nix`/source materialization.
- [ ] [depends:Phase 5] Update or archive related VMCI dogfood changes only after the live retry provides final evidence.
