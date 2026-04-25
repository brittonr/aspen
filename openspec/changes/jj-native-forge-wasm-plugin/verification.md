# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-forge-handler/src/executor.rs`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-repo-lifecycle-integration-tests.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-rustfmt-check.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-openspec-preflight.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/tasks.md`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/verification.md`

## Task Coverage

- [x] 1.1 Extend Forge repo metadata, repository list responses, and client-facing capability discovery so a repo can declare `git`, `jj`, or both backends and return node-specific routing identifiers for active backends.
  - Evidence: `crates/aspen-forge-protocol/src/lib.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 1.2 Update repository create/delete flows so JJ-enabled repos allocate JJ namespaces on create and remove/tombstone JJ namespaces, change-id indexes, and discovery metadata on delete while leaving shared blobs on the normal retention/GC path.
  - Evidence: `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-forge-repo-lifecycle-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-client-api-wire-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-rustfmt-check.txt`

- [x] 1.3 Default or backfill pre-existing repositories with no backend manifest to `git`-only behavior.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `crates/aspen-forge-protocol/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 1.6 Reject protocol-identifier collisions during plugin registration/activation and surface a deterministic error.
  - Evidence: `crates/aspen-plugin-api/src/manifest.rs`, `crates/aspen-plugin-api/src/resolve.rs`, `crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-cli-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 2.1 Define the native JJ object model and blob encoding used by Forge for commits, trees, files, conflicts, and related metadata.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-aspen-forge-jj-object-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-rustfmt-check.txt`

- [x] 2.2 Implement repo-scoped JJ object persistence and reachability lookup on top of BLAKE3-addressed blobs.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-aspen-forge-jj-store-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-rustfmt-check.txt`

- [x] 2.4 Implement Raft-backed JJ change-id indexes and JJ bookmark namespaces, including create/move/delete semantics.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-aspen-forge-jj-index-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-rustfmt-check.txt`

- [x] 2.7 Add optimistic final-publish conflict checks for stale bookmark heads and stale change-id heads.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-aspen-forge-jj-conflict-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-rustfmt-check.txt`

- [x] 2.6 Validate incoming JJ payloads and reject malformed or inconsistent object graphs before final publish.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-aspen-forge-jj-graph-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-rustfmt-check.txt`

- [x] 2.5 Implement staged JJ push publication so partial or failed streams do not make repo-visible state inconsistent.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-aspen-forge-jj-staged-publish-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-rustfmt-check.txt`

- [x] 1.7 Persist the manifest-declared JJ routing identifier in plugin registration metadata, surface it in capability discovery, and keep it stable across reload/upgrade when unchanged.
  - Evidence: `crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

- [x] 1.8 Publish and withdraw node-local JJ activation state so discovery only advertises nodes with an active JJ plugin.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

- [x] 2.8 Add staged-data quota, expiry, and cleanup behavior for successful publish, timeout, rejection, and abandoned-session paths.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-aspen-forge-jj-staged-cleanup-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-rustfmt-check.txt`

- [x] 2.9 Abort or reject in-flight JJ sessions when a repo is deleted or JJ support is disabled, and block final publish after that transition.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-aspen-forge-jj-repo-state-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-rustfmt-check.txt`

- [x] 1.4 Add JJ-native request/response families, transport identifiers, and explicit transport-version advertisement/compatibility checks for clone, fetch, push, bookmark sync, and change-id lookup.
  - Evidence: `crates/aspen-forge-protocol/src/lib.rs`, `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/request_metadata.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs`, `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-jj-native-admission-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

- [x] 3.6 Reject unsupported JJ access explicitly for Git-only repos and reject silent fallback to Git transport.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-6-jj-native-no-fallback-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-6-rustfmt-check.txt`

- [x] 3.7 Return `capability-unavailable` for JJ-enabled repos when the selected target node does not currently have the JJ plugin active, and omit JJ routing identifiers for that node from discovery.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-jj-plugin-inactive-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-rustfmt-check.txt`

- [x] 3.8 Enforce existing Forge authentication and repo read/write authorization rules on JJ-native clone, fetch, push, bookmark sync, and change-id resolution.
  - Evidence: `crates/aspen-client-api/src/messages/to_operation/forge_ops.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-jj-authz-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-rustfmt-check.txt`

- [x] 4.3 Document operator and developer workflows for creating JJ-enabled repos and connecting JJ clients.
  - Evidence: `docs/jj-native-forge.md`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-3-docs-check.txt`

- [x] 5.1 Add integration tests for Forge repo create/list/delete flows with backend manifests, repository list responses that report enabled backends, pre-existing repos that default to `git` when the manifest is absent, node-specific capability-discovery responses for Git-only, JJ-only, and dual-backend repos, positive routing-identifier presence on active JJ nodes, explicit post-delete rejection of both Git and JJ operations, post-delete unreachability of JJ discovery metadata, and shared-blob retention on the normal GC path.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-repo-lifecycle-integration-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-rustfmt-check.txt`

- [x] 1.5 Extend the plugin/runtime manifest and routing path so a WASM plugin can claim a bounded repo protocol session, not only unary requests.
  - Evidence: `crates/aspen-plugin-api/src/manifest.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-5-plugin-session-route-tests.txt`

- [x] 4.1 Support JJ-only and dual-backend repos without collisions between Git refs and JJ bookmark/change namespaces.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-dual-backend-namespace-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-rustfmt-check.txt`

- [x] 4.2 Add feature/config wiring and plugin-loading paths so JJ support can be enabled deliberately per deployment and per repo.
  - Evidence: `Cargo.toml`, `crates/aspen-cli/src/bin/aspen-cli/commands/git.rs`, `docs/jj-native-forge.md`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-cli-backend-config-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-feature-docs-check.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-rustfmt-check.txt`

- [x] 5.3 Add regression tests for change-id preservation, rewrite/update behavior, malformed-payload rejection, staged-publish no-partial-visibility, concurrent final-publish conflict rejection, repo-delete session abort/final-publish blocking, and dual-backend namespace isolation.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-3-jj-regression-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-dual-backend-namespace-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-3-rustfmt-check.txt`

- [x] 5.4 Add negative tests proving JJ-to-Git-only and Git-to-JJ-only access fail with capability errors, JJ-enabled repos on plugin-inactive target nodes return `capability-unavailable`, discovery omits JJ routing identifiers for those nodes, unauthorized JJ writes are rejected, and no path falls back silently.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `crates/aspen-client-api/src/messages/to_operation/forge_ops.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-4-git-jj-negative-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-jj-plugin-inactive-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-jj-authz-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-4-rustfmt-check.txt`

- [x] 2.3 Implement cross-node JJ blob fetch through Aspen's blob distribution path so peers can satisfy missing-object reads.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-blob/src/traits/read.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-3-jj-blob-fetch-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-3-rustfmt-check.txt`

## Review Scope Snapshot

Review remediation after same-family review: tasks 1.1, 1.4, and 1.6 are intentionally unchecked until routing discovery, session-admission compatibility checks, and lifecycle registration/activation enforcement have durable implementation evidence.

## Verification Commands

### `cargo test -p aspen-plugin-api -p aspen-forge-protocol -p aspen-forge-handler`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`

### `cargo test -p aspen-cli --features plugins-rpc <focused plugin tests>`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-cli-plugin-tests.txt`

### `rustfmt --check crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs crates/aspen-forge-handler/src/executor.rs crates/aspen-forge-protocol/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler test_execute_ -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-forge-repo-lifecycle-tests.txt`

### `cargo test -p aspen-client-api --test wire_format_golden -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-client-api-wire-tests.txt`

### `rustfmt --check <task 1.2 changed Rust files>`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-2-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-aspen-forge-jj-object-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-aspen-forge-jj-store-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-aspen-forge-jj-index-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-aspen-forge-jj-conflict-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-aspen-forge-jj-graph-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-aspen-forge-jj-staged-publish-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler active_backend_routes`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-aspen-forge-jj-staged-cleanup-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-aspen-forge-jj-repo-state-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-openspec-preflight.txt`

### `cargo test -p aspen-forge-protocol jj_native && cargo test -p aspen-forge-handler jj_native_admission && cargo test -p aspen-client-api request_metadata && cargo test -p aspen-client-api client_rpc_postcard_baseline`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-jj-native-admission-tests.txt`

### `rustfmt --check --config skip_children=true crates/aspen-forge-protocol/src/lib.rs crates/aspen-client-api/src/messages/mod.rs crates/aspen-client-api/src/messages/request_metadata.rs crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler jj_native_admission`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-6-jj-native-no-fallback-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-3-6-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler plugin -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-jj-plugin-inactive-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-7-openspec-preflight.txt`

### `cargo test -p aspen-client-api jj_native_*_operations_require_forge_*_auth`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-jj-authz-tests.txt`

### `rustfmt --check crates/aspen-client-api/src/messages/to_operation/forge_ops.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/3-8-openspec-preflight.txt`

### `test -s docs/jj-native-forge.md`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-3-docs-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-3-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler test_repo_backend_lifecycle_integration_covers_delete_and_blob_retention -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-repo-lifecycle-integration-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-1-openspec-preflight.txt`

### `cargo test -p aspen-plugin-api protocol_manifest_roundtrips && cargo test -p aspen-forge-handler test_backend_route_helpers_advertise_git_and_active_jj`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-5-plugin-session-route-tests.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-5-openspec-preflight.txt`

### `cargo test -p aspen-forge jj_namespace_keys_do_not_collide_with_git_refs && cargo test -p aspen-forge-handler test_repo_info_jj_only_omits_git_route`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-dual-backend-namespace-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs crates/aspen-forge/src/jj.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-1-openspec-preflight.txt`

### `cargo test -p aspen-cli repo --features plugins-rpc -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-cli-backend-config-tests.txt`

### `rg -n "jj-native-forge|--backend jj|--backend git,jj" docs/jj-native-forge.md && rg -n "jj-native-forge" Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-feature-docs-check.txt`

### `rustfmt --check crates/aspen-cli/src/bin/aspen-cli/commands/git.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/4-2-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-3-jj-regression-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-3-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-3-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler git_ --features git-bridge -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-4-git-jj-negative-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/5-4-openspec-preflight.txt`

### `cargo test -p aspen-forge jj_object_store_fetches_object_blob_by_hash`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-3-jj-blob-fetch-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-3-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-3-openspec-preflight.txt`

## Notes

- Remaining unchecked tasks are the plugin/runtime session path, standalone `jj-remote-aspen`, authz, feature/config wiring, and broader integration/negative transport coverage.
- Same-family review correction is tracked in `openspec/changes/jj-native-forge-wasm-plugin/evidence/review-remediation-task-scope.md`.
