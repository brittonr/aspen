Evidence-ID: extend-no-std-foundation-and-wire.v4
Task-ID: V4
Artifact-Type: verification-plan
Covers: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable

# Wire compatibility verification plan

Planned evidence for task `V4`:

- `cargo test -p aspen-client-api`
- `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture`
- saved output proving the postcard regression tests use alloc-safe serializers
- saved output from the discriminant/default-feature regression tests already in `crates/aspen-client-api/src/lib.rs`
- a deterministic baseline artifact at `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json` capturing at least one variant-keyed default-production postcard encoding for every `ClientRpcRequest` / `ClientRpcResponse` enum variant, using canonical payload values derived only from fixed strings, fixed integers, fixed bytes, deterministic single-entry collections, and recursively canonical nested values
- saved output showing the `client_rpc_postcard_baseline` test first checks completeness against the live enum set and then matches that baseline artifact byte-for-byte
- representative runtime consumer compile checks for `aspen-cluster`, `aspen-client`, `aspen-cli`, `aspen-rpc-handlers`, and root `aspen` via `--no-default-features --features node-runtime` after the refactor
- saved command transcripts under this change's evidence directory

Final implementation stores the concrete command transcripts and test outputs alongside this plan.
