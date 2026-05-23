## Why

The latest `nix run .#dogfood-local-vmci -- full` retry moved past VMCI startup, guest registration, direct-route retention, job assignment, executor start, and job-result publication, then failed inside the VM job because the materialized workspace did not contain `flake.nix`:

```text
path '/tmp/workspaces/445758f2-5f3c-413b-afa4-de3cdb9e9d7f' does not contain a 'flake.nix'
error: could not find a flake.nix file
```

The likely boundary is the RPC-triggered CI path: `handle_trigger_pipeline` checks out Forge source and starts the orchestrator with `source_hash: None`, while VM workers require `source_hash` to seed their isolated workspace from the blob store. The adapter-triggered path already creates a source archive and stores its hash in `PipelineContext`; the RPC path must do the same.

## What Changes

- **RPC CI source archive propagation**: Create a source archive for the checked-out Forge tree during `CiTriggerPipeline` handling when the orchestrator has a blob store, and pass the resulting `source_hash` into `PipelineContext`.
- **VM workspace fail-fast evidence**: If a VM/local executor job requires source materialization, failure should identify missing source archive propagation or missing `flake.nix` after materialization rather than only surfacing a generic nix flake error.
- **Regression coverage**: Add focused tests that the RPC trigger context carries a `source_hash`, and that VM-transformed nix payloads preserve it through local executor workspace seeding.

## Capabilities

### Modified Capabilities
- `ci-forge-nix-compat`: RPC-triggered Forge/Nix CI jobs must produce a VM-materializable source archive whose workspace root contains `flake.nix`.
- `federation-mirror-ci-trigger`: `CiTriggerPipeline` regression coverage must include source archive propagation, not only successful run creation.
- `dogfood-evidence`: VM-CI diagnostics must classify missing workspace source/flake evidence as source materialization failure.

## Impact

- **Files**: expected implementation near `crates/aspen-ci-handler/src/handler/pipeline.rs`, source archive helpers in `aspen-ci`/`aspen-ci-executor-shell`, VM worker diagnostics in `src/bin/aspen_node/worker_only.rs` only if needed, and OpenSpec task/evidence files.
- **APIs**: no external API change expected; internal helper signatures may accept `source_hash` or blob-store context.
- **Dependencies**: no new dependency expected.
- **Testing**: focused unit/integration tests for RPC trigger source archive propagation; `nix run .#rustfmt`; relevant `cargo test`; `openspec validate propagate-rpc-ci-source-archive --strict --json`; final `nix run .#dogfood-local-vmci -- full` retry.