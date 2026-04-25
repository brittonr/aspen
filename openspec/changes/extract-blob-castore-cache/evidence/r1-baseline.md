# R1 baseline: blob/castore/cache extraction

Captured: 2026-04-25T18:42:56Z

## Worktree setup

- Cargo path dependencies use local .pi/worktrees sibling mirrors for aspen-dns and aspen-wasm-plugin so their Aspen path dependencies resolve to this session worktree.

## Scope

Core crates: aspen-blob, aspen-castore, aspen-cache.
Representative consumers: aspen-rpc-core --features blob, aspen-blob-handler, aspen-snix, aspen-snix-bridge, aspen-nix-cache-gateway, aspen-ci-executor-nix.

## Command matrix

| Command | Exit | Artifact |
|---|---:|---|
| cargo check -p aspen-blob --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-blob.txt |
| cargo check -p aspen-castore --locked | 101 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-castore.txt |
| cargo check -p aspen-cache --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-cache.txt |
| cargo check -p aspen-rpc-core --features blob --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-rpc-core-blob.txt |
| cargo check -p aspen-blob-handler --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-blob-handler.txt |
| cargo check -p aspen-snix --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-snix.txt |
| cargo check -p aspen-snix-bridge --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-snix-bridge.txt |
| cargo check -p aspen-nix-cache-gateway --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-nix-cache-gateway.txt |
| cargo check -p aspen-ci-executor-nix --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-ci-executor-nix.txt |
| cargo tree -p aspen-blob -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_blob-normal.txt |
| cargo tree -p aspen-blob -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_blob-features.txt |
| cargo tree -p aspen-castore -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_castore-normal.txt |
| cargo tree -p aspen-castore -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_castore-features.txt |
| cargo tree -p aspen-cache -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_cache-normal.txt |
| cargo tree -p aspen-cache -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_cache-features.txt |
| cargo tree -p aspen-rpc-core -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_rpc_core-normal.txt |
| cargo tree -p aspen-rpc-core -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_rpc_core-features.txt |
| cargo tree -p aspen-blob-handler -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_blob_handler-normal.txt |
| cargo tree -p aspen-blob-handler -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_blob_handler-features.txt |
| cargo tree -p aspen-snix -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_snix-normal.txt |
| cargo tree -p aspen-snix -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_snix-features.txt |
| cargo tree -p aspen-snix-bridge -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_snix_bridge-normal.txt |
| cargo tree -p aspen-snix-bridge -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_snix_bridge-features.txt |
| cargo tree -p aspen-nix-cache-gateway -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_nix_cache_gateway-normal.txt |
| cargo tree -p aspen-nix-cache-gateway -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_nix_cache_gateway-features.txt |
| cargo tree -p aspen-ci-executor-nix -e normal --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_ci_executor_nix-normal.txt |
| cargo tree -p aspen-ci-executor-nix -e features --depth 3 --locked | 0 | openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-tree-aspen_ci_executor_nix-features.txt |

## Direct dependency classification

### aspen-blob

| Section | Dependency | Class | Notes |
|---|---|---|---|
| dependencies | anyhow | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | aspen-client-api | adapter-runtime | Aspen integration coupling to evaluate/gate |
| dependencies | aspen-core | adapter-runtime | Aspen integration coupling to evaluate/gate |
| dependencies | async-trait | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | bytes | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | hex | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | iroh | backend-purpose | declared backend/domain dependency |
| dependencies | iroh-blobs | backend-purpose | declared backend/domain dependency |
| dependencies | n0-future | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | parking_lot | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | postcard | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | serde | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | serde_json | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | snafu | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tokio | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tokio-util | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tracing | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dev-dependencies | rand | test-only | non-production dependency |

### aspen-cache

| Section | Dependency | Class | Notes |
|---|---|---|---|
| dependencies | aspen-blob | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | aspen-core | adapter-runtime | Aspen integration coupling to evaluate/gate |
| dependencies | aspen-kv-types | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | async-trait | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | data-encoding | backend-purpose | declared backend/domain dependency |
| dependencies | ed25519-dalek | backend-purpose | declared backend/domain dependency |
| dependencies | nix-compat | backend-purpose | declared backend/domain dependency |
| dependencies | rand_core_06 | backend-purpose | declared backend/domain dependency |
| dependencies | serde | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | serde_json | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | sha2 | backend-purpose | declared backend/domain dependency |
| dependencies | snafu | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tokio | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tracing | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dev-dependencies | aspen-testing | test-only | non-production dependency |
| dev-dependencies | tempfile | test-only | non-production dependency |
| dev-dependencies | tokio | test-only | non-production dependency |

### aspen-castore

| Section | Dependency | Class | Notes |
|---|---|---|---|
| dependencies | aspen-blob | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | aspen-core | adapter-runtime | Aspen integration coupling to evaluate/gate |
| dependencies | async-stream | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | async-trait | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | futures | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | hex | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | iroh | backend-purpose | declared backend/domain dependency |
| dependencies | irpc | backend-purpose | declared backend/domain dependency |
| dependencies | n0-error | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | n0-future | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | postcard | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | prost | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | serde | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | snafu | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | snix-castore | backend-purpose | declared backend/domain dependency |
| dependencies | tokio | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dependencies | tracing | reusable-domain | library/runtime utility or reusable Aspen domain dependency |
| dev-dependencies | aspen-blob | test-only | non-production dependency |
| dev-dependencies | futures | test-only | non-production dependency |
| dev-dependencies | nix-compat | test-only | non-production dependency |
| dev-dependencies | sha2 | test-only | non-production dependency |
| dev-dependencies | snix-store | test-only | non-production dependency |
| dev-dependencies | tempfile | test-only | non-production dependency |
| dev-dependencies | tokio | test-only | non-production dependency |
| dev-dependencies | tokio-stream | test-only | non-production dependency |

## Baseline coupling observations

- aspen-blob default currently depends on aspen-client-api for replication RPC and aspen-core for KV-backed metadata; this is the primary adapter/runtime coupling for I3.
- aspen-castore default currently depends on aspen-core for circuit-breaker behavior; this is the I4 seam.
- aspen-cache default currently depends on aspen-core, aspen-kv-types, and aspen-blob; dev tests pull aspen-testing, which should remain test-only or move to shell integration coverage.
- Representative consumer compile/check status is recorded in the command matrix above; failures are baseline facts, not regressions from this task.
